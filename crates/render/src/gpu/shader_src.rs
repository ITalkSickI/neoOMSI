pub(crate) fn scene_shader_source() -> String {
    [
        include_str!("../../shaders/scene/scene_base.wgsl"),
        include_str!("../../shaders/enhanced/common_lighting.wgsl"),
        include_str!("../../shaders/puddles/puddle_common.wgsl"),
        include_str!("../../shaders/enhanced/scene_lighting.wgsl"),
    ]
        .join("\n")
}

pub(crate) fn sky_shader_source() -> String {
    [
        include_str!("../../shaders/sky/sky_base.wgsl"),
        include_str!("../../shaders/enhanced/common_lighting.wgsl"),
        include_str!("../../shaders/enhanced/sky_lighting.wgsl"),
    ]
    .join("\n")
}

pub(crate) fn corona_shader_source() -> String {
    [
        include_str!("../../shaders/sky/corona.wgsl"),
        include_str!("../../shaders/enhanced/common_lighting.wgsl"),
    ]
    .join("\n")
}
