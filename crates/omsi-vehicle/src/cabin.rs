//! `passengercabin.cfg` (unit `mc_passcabin`).

use omsi_cfg::CfgFile;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct PassPos {
    pub pos: [f32; 3],
    pub height: f32,
    pub rot: f32,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Entry {
    pub path_point: i32,
    pub no_ticket_sale: bool,
    pub with_button: bool,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Point3 {
    pub path_point: i32,
    pub pos: [f32; 3],
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct VarPoint {
    pub pos: [f32; 3],
    pub var: [f32; 2],
    pub parent: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct PassengerCabin {
    pub entries: Vec<Entry>,
    pub exits: Vec<i32>,
    pub link_to_next_veh: Option<i32>,
    pub link_to_prev_veh: Option<i32>,
    pub stampers: Vec<Point3>,
    pub ticket_sales: Vec<Point3>,
    pub money_points: Vec<VarPoint>,
    pub change_points: Vec<VarPoint>,
    pub pass_positions: Vec<PassPos>,
    pub driver_positions: Vec<PassPos>,
    pub illumination_interior: Vec<i32>,
}

impl PassengerCabin {
    pub fn load(path: &Path) -> Result<PassengerCabin, omsi_cfg::CfgError> {
        let f = CfgFile::read(path)?;
        Ok(Self::parse(&f))
    }

    pub fn parse(f: &CfgFile) -> PassengerCabin {
        let mut c = PassengerCabin::default();
        let mut r = f.reader().disabled_blocks();
        while let Some(k) = r.next_keyword() {
            match k.as_str() {
                "entry" => {
                    let mut e = Entry { path_point: r.i32(), ..Default::default() };
                    loop {
                        let save = r.pos();
                        let w = r.word().to_ascii_lowercase();
                        match w.as_str() {
                            "{noticketsale}" => e.no_ticket_sale = true,
                            "{withbutton}" => e.with_button = true,
                            _ => {
                                r.seek(save);
                                break;
                            }
                        }
                    }
                    c.entries.push(e);
                }
                "exit" => c.exits.push(r.i32()),
                "linktonextveh" => c.link_to_next_veh = Some(r.i32()),
                "linktoprevveh" => c.link_to_prev_veh = Some(r.i32()),
                "stamper" => c.stampers.push(Point3 { path_point: r.i32(), pos: r.f32s::<3>() }),
                "ticket_sale" => c.ticket_sales.push(Point3 { path_point: r.i32(), pos: r.f32s::<3>() }),
                "ticket_sale_money_point" | "ticket_sale_money_point_2" | "ticket_sale_change_point" | "ticket_sale_change_point_2" => {
                    let pos = r.f32s::<3>();
                    let var = r.f32s::<2>();
                    let parent = if k.ends_with("_2") { Some(r.str().to_string()) } else { None };
                    let p = VarPoint { pos, var, parent };
                    if k.contains("money") {
                        c.money_points.push(p);
                    } else {
                        c.change_points.push(p);
                    }
                }
                "passpos" | "drivpos" => {
                    let pos = r.f32s::<3>();
                    let height = r.f32();
                    let rot = r.f32();
                    let p = PassPos { pos, height, rot };
                    if k == "passpos" {
                        c.pass_positions.push(p);
                    } else {
                        c.driver_positions.push(p);
                    }
                }
                "illumination_interior" => c.illumination_interior = r.i32_list(4),
                _ => {}
            }
        }
        c
    }
}
