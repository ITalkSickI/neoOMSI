//! `paths.cfg` - passenger walking paths inside a vehicle.

use omsi_cfg::CfgFile;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct PathPoint {
    pub pos: [f32; 3],
    pub room_height: f32,
    pub step_sound: i32,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct VehiclePaths {
    pub step_sound_packs: Vec<Vec<String>>,
    pub points: Vec<PathPoint>,
    /// (a, b, one_way)
    pub links: Vec<(i32, i32, bool)>,
}

impl VehiclePaths {
    pub fn load(path: &Path) -> Result<VehiclePaths, omsi_cfg::CfgError> {
        let f = CfgFile::read(path)?;
        Ok(Self::parse(&f))
    }

    pub fn parse(f: &CfgFile) -> VehiclePaths {
        let mut p = VehiclePaths::default();
        let mut room_height = 2.0;
        let mut step_sound = 0;
        let mut r = f.reader().disabled_blocks();
        while let Some(k) = r.next_keyword() {
            match k.as_str() {
                "stepsoundpack" => {
                    let n = r.usize();
                    p.step_sound_packs.push((0..n).map(|_| r.str().to_string()).collect());
                }
                "next_roomheight" => room_height = r.f32(),
                "next_stepsound" => step_sound = r.i32(),
                "pathpnt" => p.points.push(PathPoint { pos: r.f32s::<3>(), room_height, step_sound }),
                "pathlink" => {
                    let a = r.i32();
                    let b = r.i32();
                    p.links.push((a, b, false));
                }
                "pathlink_oneway" => {
                    let a = r.i32();
                    let b = r.i32();
                    p.links.push((a, b, true));
                }
                _ => {}
            }
        }
        p
    }
}
