//! The paper timetable in the driver's cab.
//!
//! Stock vehicle models show this through `[matl_freetex] file_schedule`. OMSI supplies the
//! bitmap named by that string; openOMSI makes it from the player's current duty and keeps
//! it in its own cache so the original installation remains read-only.

use crate::schedule::{PlannedStop, PlayerDuty};
use anyhow::{anyhow, Context, Result};
use omsi_content::font::{FontAtlas, TextAlign};
use omsi_sim::VehicleInstance;
use omsi_texture::Image;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

const PAPER_X: u32 = 100;
const TIME_X: u32 = 650;
const PAPER_TOP: u32 = 88;
const ROWS_TOP_GAP: u32 = 20;
const PAPER_BOTTOM_MARGIN: u32 = 48;
const TEXT_COLOR: [u8; 3] = [17, 15, 14];

#[derive(Debug, Clone, PartialEq, Eq)]
struct PaperRow {
    name: String,
    time: String,
}

/// Update `file_schedule` to a cached drawing of the current trip. The renderer already
/// handles the model's `[matl_freetex]` slot, so switching this string updates the paper.
pub(crate) fn update_vehicle(
    vehicle: &mut VehicleInstance,
    duty: &PlayerDuty,
    fonts: &mut omsi_sim::texttex::FontLibrary,
) -> Result<()> {
    let (title, rows) = paper_content(&duty.line, &duty.tour, &duty.trips, duty.trip_index);
    let signature = content_signature(&title, &rows);
    let path = cache_dir()?.join(format!("schedule-v2-{signature:016x}.png"));
    let filename = path.to_string_lossy().into_owned();

    if vehicle.str_var("file_schedule") == filename {
        return Ok(());
    }

    if !path.is_file() {
        let Some(font) = schedule_font(fonts) else {
            set_filename(vehicle, "");
            return Err(anyhow!(
                "no OMSI bitmap font is available for the driver's timetable"
            ));
        };
        let mut image = paper_base();
        draw_schedule(&mut image, &font, &title, &rows);
        save_png(&image, &path)?;
    }
    set_filename(vehicle, &filename);
    Ok(())
}

fn cache_dir() -> Result<PathBuf> {
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .ok_or_else(|| anyhow!("cannot locate the openOMSI user data folder"))?;
    Ok(home.join(".openomsi").join("cache").join("schedules"))
}

fn set_filename(vehicle: &mut VehicleInstance, value: &str) {
    if let Some(i) = vehicle.ty.program.str_var("file_schedule") {
        vehicle.state.str_vars[i as usize] = value.to_string();
    } else {
        log::debug!(
            "{} has no file_schedule string variable",
            vehicle.ty.def.type_name
        );
    }
}

fn schedule_font(fonts: &mut omsi_sim::texttex::FontLibrary) -> Option<std::sync::Arc<FontAtlas>> {
    ["19_HHAschedule_font", "DIN Narrow", "DIN_Narrow", "DIN"]
        .into_iter()
        .find_map(|name| fonts.load(name))
}

fn paper_content(
    duty_line: &str,
    duty_tour: &str,
    trips: &[crate::schedule::PlannedTrip],
    trip_index: usize,
) -> (String, Vec<PaperRow>) {
    let trip = &trips[trip_index];
    let line = if trip.line.trim().is_empty() {
        duty_line.trim()
    } else {
        trip.line.trim()
    };
    let title = format!("{line} - {} - {}", trip.terminus.trim(), duty_tour.trim());

    let served: Vec<(usize, &PlannedStop)> = trip
        .stops
        .iter()
        .enumerate()
        .filter(|(_, stop)| stop.stops)
        .collect();
    let last = served.last().map(|(index, _)| *index);
    let mut rows: Vec<PaperRow> = served
        .iter()
        .map(|(index, stop)| {
            let arrival = *index == last.unwrap_or(usize::MAX);
            let name = if arrival {
                format!("{} Ankunft", stop.name.trim())
            } else {
                stop.name.trim().to_string()
            };
            let time = if arrival {
                format_time(stop.arr)
            } else if stop.dep - stop.arr >= 60.0 {
                format!("{} - {}", format_time(stop.arr), format_time(stop.dep))
            } else {
                format_time(stop.dep)
            };
            PaperRow { name, time }
        })
        .collect();

    if let (Some((_, final_stop)), Some(next)) = (served.last(), trips.get(trip_index + 1)) {
        let next_start = next
            .stops
            .iter()
            .find(|stop| stop.stops)
            .or_else(|| next.stops.first());
        let same_stop = next_start.is_some_and(|next_stop| {
            final_stop.object_id == next_stop.object_id
                || (!final_stop.name.trim().is_empty()
                    && final_stop
                        .name
                        .trim()
                        .eq_ignore_ascii_case(next_stop.name.trim()))
        });
        if same_stop {
            rows.push(PaperRow {
                name: "Abfahrt".into(),
                time: format_time(next.departure),
            });
        }
    }
    (title, rows)
}

fn format_time(seconds: f64) -> String {
    let minute = (seconds / 60.0).round() as i64;
    let minute = minute.rem_euclid(24 * 60);
    format!("{:02}:{:02}", minute / 60, minute % 60)
}

fn content_signature(title: &str, rows: &[PaperRow]) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    title.hash(&mut hasher);
    for row in rows {
        row.name.hash(&mut hasher);
        row.time.hash(&mut hasher);
    }
    hasher.finish()
}

fn paper_base() -> Image {
    let dirs = omsi_cfg::content_dirs("Texture");
    let refs: Vec<&Path> = dirs.iter().map(PathBuf::as_path).collect();
    omsi_texture::find_texture("Schedule.bmp", &refs)
        .and_then(|path| omsi_texture::decode_file(&path).ok())
        .unwrap_or_else(|| Image {
            width: 1024,
            height: 1024,
            rgba: [230, 227, 216, 255]
                .into_iter()
                .cycle()
                .take(1024 * 1024 * 4)
                .collect(),
            has_alpha: false,
        })
}

fn draw_schedule(image: &mut Image, font: &FontAtlas, title: &str, rows: &[PaperRow]) {
    let native_height = font.font.height.max(1) as u32;
    let scale = fit_scale(image.height, native_height, rows.len());
    let line_height = scaled_height(native_height, scale);
    let time_x = PAPER_X + scaled_size(TIME_X - PAPER_X, scale);
    draw_text(image, font, title, PAPER_X, PAPER_TOP, scale);

    let equal_width = scaled_text_width(font, "=", scale).max(1);
    let right_margin = scaled_size(130, scale);
    let count =
        (image.width.saturating_sub(PAPER_X + right_margin) / equal_width).min(120) as usize;
    draw_text(
        image,
        font,
        &"=".repeat(count),
        PAPER_X,
        PAPER_TOP + line_height + 4,
        scale,
    );

    let mut y = PAPER_TOP + native_height + ROWS_TOP_GAP;
    for row in rows {
        if y + line_height >= image.height {
            break;
        }
        let name_width = scaled_text_width(font, &row.name, scale);
        let dot_width = scaled_text_width(font, ".", scale).max(1);
        let available = time_x.saturating_sub(PAPER_X + name_width + scaled_size(12, scale));
        let dots = (available / dot_width).min(96) as usize;
        draw_text(image, font, &row.name, PAPER_X, y, scale);
        draw_text(
            image,
            font,
            &".".repeat(dots),
            PAPER_X + name_width + scaled_size(9, scale),
            y,
            scale,
        );
        draw_text(image, font, &row.time, time_x, y, scale);
        y += line_height;
    }
}

fn fit_scale(image_height: u32, line_height: u32, row_count: usize) -> f32 {
    if row_count == 0 {
        return 1.0;
    }
    let rows_top = PAPER_TOP + line_height + ROWS_TOP_GAP;
    let available = image_height.saturating_sub(rows_top + PAPER_BOTTOM_MARGIN);
    ((available as f32 / (row_count as f32 * line_height as f32)).min(1.0)).max(0.01)
}

fn scaled_size(size: u32, scale: f32) -> u32 {
    ((size as f32 * scale).round() as u32).max(1)
}

fn scaled_height(size: u32, scale: f32) -> u32 {
    ((size as f32 * scale).floor() as u32).max(1)
}

fn scaled_text_width(font: &FontAtlas, text: &str, scale: f32) -> u32 {
    scaled_size(font.text_width(text).max(0) as u32, scale)
}

fn draw_text(image: &mut Image, font: &FontAtlas, text: &str, x: u32, y: u32, scale: f32) {
    if x >= image.width || y >= image.height {
        return;
    }
    let source_width = (font.text_width(text).max(0) as u32).max(1);
    let source_height = font.font.height.max(1) as u32;
    let rgba = font.render_aligned(
        text,
        source_width,
        source_height,
        false,
        TEXT_COLOR,
        TextAlign {
            orientation: 1,
            grid: 1,
        },
    );
    let width = scaled_size(source_width, scale).min(image.width - x);
    let height = scaled_height(source_height, scale).min(image.height - y);
    for py in 0..height {
        for px in 0..width {
            let source_x = ((px as f32 / scale).floor() as u32).min(source_width - 1);
            let source_y = ((py as f32 / scale).floor() as u32).min(source_height - 1);
            let source = ((source_y * source_width + source_x) * 4) as usize;
            let alpha = rgba[source + 3] as u32;
            if alpha == 0 {
                continue;
            }
            let target = (((y + py) * image.width + x + px) * 4) as usize;
            for channel in 0..3 {
                let ink = TEXT_COLOR[channel] as u32;
                let paper = image.rgba[target + channel] as u32;
                image.rgba[target + channel] = ((ink * alpha + paper * (255 - alpha)) / 255) as u8;
            }
            image.rgba[target + 3] = 255;
        }
    }
}

fn save_png(image: &Image, path: &Path) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| anyhow!("schedule cache path has no parent"))?;
    std::fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
    let buffer = image::RgbaImage::from_raw(image.width, image.height, image.rgba.clone())
        .ok_or_else(|| anyhow!("schedule image has the wrong pixel count"))?;
    buffer
        .save(path)
        .with_context(|| format!("writing {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schedule::{PlannedTrip, StopDir};
    use omsi_content::font::FontChar;

    fn stop(id: i64, name: &str, arr: f64, dep: f64) -> PlannedStop {
        PlannedStop {
            object_id: id,
            name: name.into(),
            arr,
            dep,
            position: None,
            dir: StopDir::default(),
            stops: true,
        }
    }

    fn test_font() -> FontAtlas {
        let chars: Vec<_> = (32u8..=126)
            .enumerate()
            .map(|(index, byte)| FontChar {
                ch: byte as char,
                x0: index as i32 * 4,
                x1: (index as i32 + 1) * 4,
                y: 0,
            })
            .collect();
        let width = chars.len() as u32 * 4;
        let pixels = vec![255; (width * 27 * 4) as usize];
        FontAtlas::new(
            omsi_content::font::Font {
                height: 27,
                gap: 1,
                chars,
                ..Default::default()
            },
            width,
            27,
            pixels.clone(),
            pixels,
        )
    }

    #[test]
    fn paper_keeps_repeated_stops_and_adds_terminal_departure() {
        let current = PlannedTrip {
            name: "76_Kk-BH".into(),
            line: "76".into(),
            terminus: "Bauernhof".into(),
            departure: 11.0 * 3600.0 + 52.0 * 60.0,
            end: 11.0 * 3600.0 + 59.0 * 60.0,
            stops: vec![
                stop(
                    1,
                    "Krankenhaus",
                    11.0 * 3600.0 + 52.0 * 60.0,
                    11.0 * 3600.0 + 52.0 * 60.0,
                ),
                stop(
                    2,
                    "Krankenhaus",
                    11.0 * 3600.0 + 52.0 * 60.0,
                    11.0 * 3600.0 + 52.0 * 60.0,
                ),
                stop(
                    3,
                    "Bauernhof",
                    11.0 * 3600.0 + 59.0 * 60.0,
                    11.0 * 3600.0 + 59.0 * 60.0,
                ),
            ],
        };
        let next = PlannedTrip {
            name: "76_BH-Kk".into(),
            line: "76".into(),
            terminus: "Krankenhaus".into(),
            departure: 12.0 * 3600.0 + 7.0 * 60.0,
            end: 12.0 * 3600.0 + 14.0 * 60.0,
            stops: vec![stop(
                3,
                "Bauernhof",
                12.0 * 3600.0 + 7.0 * 60.0,
                12.0 * 3600.0 + 7.0 * 60.0,
            )],
        };
        let (title, rows) = paper_content("76", "1", &[current, next], 0);
        assert_eq!(title, "76 - Bauernhof - 1");
        assert_eq!(
            rows.iter().map(|r| r.name.as_str()).collect::<Vec<_>>(),
            ["Krankenhaus", "Krankenhaus", "Bauernhof Ankunft", "Abfahrt",]
        );
        assert_eq!(
            rows.iter().map(|r| r.time.as_str()).collect::<Vec<_>>(),
            ["11:52", "11:52", "11:59", "12:07",]
        );
    }

    #[test]
    fn paper_times_wrap_after_midnight() {
        assert_eq!(format_time(24.0 * 3600.0 + 7.0 * 60.0), "00:07");
    }

    #[test]
    fn dense_schedule_scales_rows_to_keep_them_on_the_paper() {
        let scale = fit_scale(1024, 27, 36);
        let row_height = scaled_height(27, scale);
        let rows_top = PAPER_TOP + 27 + ROWS_TOP_GAP;
        assert!(scale < 1.0);
        assert!(rows_top + row_height * 36 <= 1024 - PAPER_BOTTOM_MARGIN);
        assert_eq!(fit_scale(1024, 27, 9), 1.0);

        let font = test_font();
        let rows: Vec<_> = (0..36)
            .map(|i| PaperRow {
                name: format!("Stop {i:02}"),
                time: "12:34".into(),
            })
            .collect();
        let mut image = Image {
            width: 1024,
            height: 1024,
            rgba: [255, 255, 255, 255]
                .into_iter()
                .cycle()
                .take(1024 * 1024 * 4)
                .collect(),
            has_alpha: false,
        };
        draw_schedule(&mut image, &font, "76 - Dense - 1", &rows);

        for i in 0..36 {
            let y = rows_top + i * row_height;
            let has_ink = (y..y + row_height).any(|py| {
                (PAPER_X..PAPER_X + 80).any(|px| {
                    let pixel = ((py * image.width + px) * 4) as usize;
                    image.rgba[pixel] < 100
                })
            });
            assert!(has_ink, "schedule row {i} was not rendered");
        }
    }
}
