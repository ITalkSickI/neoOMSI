//! `.sli` spline definitions (unit `mc_splines`).

use crate::PathDef;
use omsi_cfg::CfgFile;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Default)]
pub struct SplineProfilePoint {
    /// Lateral position (m, positive = right).
    pub x: f32,
    /// Height above the spline (m).
    pub z: f32,
    /// Texture u coordinate.
    pub u: f32,
    /// Texture scale along the spline (v per metre).
    pub v_scale: f32,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct SplineProfile {
    pub texture: usize,
    pub points: Vec<SplineProfilePoint>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct HeightProfile {
    pub x0: f32,
    pub x1: f32,
    pub z0: f32,
    pub z1: f32,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct PatchworkChain {
    pub segment_length: f32,
    pub chain: String,
    pub weights: String,
    pub invertable: String,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct SplineTexture {
    pub file: String,
    pub patchwork: Option<PatchworkChain>,
    pub alpha: i32,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct RailEnh {
    pub values: [f32; 8],
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ThirdRail {
    pub values: [f32; 6],
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Spline {
    pub path: PathBuf,
    pub length: f32,
    pub textures: Vec<SplineTexture>,
    pub scale_tex_by_length: bool,
    pub height_profiles: Vec<HeightProfile>,
    pub profiles: Vec<SplineProfile>,
    pub paths: Vec<PathDef>,
    pub rail_enh: Vec<RailEnh>,
    pub third_rail: Vec<ThirdRail>,
    pub half_cant_width: Option<f32>,
    pub only_editor: bool,
    pub terrain_hole_profile: Vec<[f32; 3]>,
    pub unknown_keywords: Vec<(String, usize)>,
}

impl Spline {
    pub fn load(path: &Path) -> Result<Spline, omsi_cfg::CfgError> {
        let f = CfgFile::read(path)?;
        Ok(Self::parse(&f))
    }

    pub fn parse(file: &CfgFile) -> Spline {
        let mut s = Spline { path: file.path.clone(), ..Default::default() };
        let mut r = file.reader();
        let mut in_hole = false;
        while let Some(k) = r.next_keyword() {
            match k.as_str() {
                "length" => s.length = r.f32(),
                "texture" => {
                    s.textures.push(SplineTexture { file: r.str().to_string(), ..Default::default() });
                }
                "scaletexbylength" => s.scale_tex_by_length = true,
                "patchwork_chain" => {
                    let pc = PatchworkChain { segment_length: r.f32(), chain: r.word().to_string(), weights: r.word().to_string(), invertable: r.word().to_string() };
                    if let Some(t) = s.textures.last_mut() {
                        t.patchwork = Some(pc);
                    }
                }
                "matl_alpha" => {
                    let a = r.i32();
                    if let Some(t) = s.textures.last_mut() {
                        t.alpha = a;
                    }
                }
                "heightprofile" => {
                    let v = r.f32s::<4>();
                    s.height_profiles.push(HeightProfile { x0: v[0], x1: v[1], z0: v[2], z1: v[3] });
                }
                "profile" => {
                    in_hole = false;
                    s.profiles.push(SplineProfile { texture: r.usize(), points: Vec::new() });
                }
                "profilepnt" => {
                    let v = r.f32s::<4>();
                    if let Some(p) = s.profiles.last_mut() {
                        p.points.push(SplineProfilePoint { x: v[0], z: v[1], u: v[2], v_scale: v[3] });
                    }
                }
                "path" => {
                    let kind = r.i32();
                    let x = r.f32();
                    let z = r.f32();
                    let width = r.f32();
                    let direction = r.i32();
                    s.paths.push(PathDef { kind, start: [x, 0.0, z], width, direction, ..Default::default() });
                }
                "path_2" => {
                    let kind = r.i32();
                    let x = r.f32();
                    let z = r.f32();
                    let width = r.f32();
                    let direction = r.i32();
                    let extra = r.f32();
                    s.paths.push(PathDef { kind, start: [x, 0.0, z], width, direction, params: vec![extra], ..Default::default() });
                }
                "rail_enh" => s.rail_enh.push(RailEnh { values: r.f32s::<8>() }),
                "third_rail" => s.third_rail.push(ThirdRail { values: r.f32s::<6>() }),
                "halfcantwidth" => s.half_cant_width = Some(r.f32()),
                "onlyeditor" => s.only_editor = true,
                "terrainholeprofile" => in_hole = true,
                "terrainholeprofilepnt" => {
                    let v = r.f32s::<3>();
                    if in_hole || true {
                        s.terrain_hole_profile.push(v);
                    }
                }
                _ => s.unknown_keywords.push((k, r.block_line())),
            }
        }
        s
    }
}
