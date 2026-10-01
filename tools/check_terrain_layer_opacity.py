"""GPU regression: painted ground coverage must not change with diffuse mip levels.

Run with Python package wgpu. Uses both production terrain-alpha blocks and synthetic
textures only; a lower ground layer is red so leakage is measured independently of
the green paint's changing brightness. No map or renderer build is needed.
"""
from pathlib import Path
import struct

import wgpu


ROOT = Path(__file__).resolve().parents[1] / "crates/omsi-render/src"
source = "\n".join((ROOT / n).read_text(encoding="utf-8") for n in (
    "shader.wgsl", "enhanced_common.wgsl", "enhanced.wgsl",
))
# Exercise the same alpha block that the colour pass uses, with its inputs supplied
# by this small probe rather than the full lighting/camera pipeline.
blocks = {}
for name, filename, entry in (("Vanilla", "shader.wgsl", "fs_main"),
                              ("Enhanced", "enhanced.wgsl", "fs_enhanced")):
    shader = (ROOT / filename).read_text(encoding="utf-8")
    fragment = shader[shader.index(f"fn {entry}("):]
    start = fragment.index("    if (material.params.z > 0.5) {")
    blocks[name] = fragment[start:fragment.index("    let mode = material.params.x;", start)]
probe = """
@vertex
fn probe_vertex(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> {
    let p = array<vec2<f32>, 3>(vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0), vec2<f32>(-1.0, 3.0));
    return vec4<f32>(p[i], 0.0, 1.0);
}
@fragment
fn probe_fragment() -> @location(0) vec4<f32> {
    let buv = vec2<f32>(0.5);
    let terrain = material.extra.x > 0.5;
    let msk_lod = 0.0;
    var tex = textureSampleLevel(t_diffuse, s_diffuse, buv, material.flags.x);
ALPHA_BLOCK
    return vec4<f32>(mix(vec3<f32>(1.0, 0.0, 0.0), tex.rgb, tex.a), tex.a);
}
"""
adapter = wgpu.gpu.request_adapter_sync(power_preference="low-power")
device = adapter.request_device_sync()
usage = wgpu.TextureUsage.TEXTURE_BINDING | wgpu.TextureUsage.COPY_DST
diffuse = device.create_texture(size=(8, 8, 1), mip_level_count=4,
                                format="rgba8unorm", usage=usage)
# A bright near texel becomes darker after the surrounding texels enter its footprint.
size = 8
greens = [255 if 3 <= x < 5 and 3 <= y < 5 else 0
          for y in range(size) for x in range(size)]
for level in range(4):
    device.queue.write_texture(
        {"texture": diffuse, "mip_level": level},
        bytes(c for green in greens for c in (0, green, 0, 255)),
        {"bytes_per_row": size * 4, "rows_per_image": size}, (size, size, 1),
    )
    if size > 1:
        greens = [round(sum(greens[(2*y+j)*size + 2*x+i]
                            for j in range(2) for i in range(2)) / 4)
                  for y in range(size // 2) for x in range(size // 2)]
        size //= 2
mask = device.create_texture(size=(1, 1, 1), format="rgba8unorm", usage=usage)
sampler = device.create_sampler(mag_filter="linear", min_filter="linear",
                                mipmap_filter="linear", address_mode_u="clamp-to-edge",
                                address_mode_v="clamp-to-edge")
params = [0.0] * 36
params[4:8] = [2.0, 0.0, 1.0, 1.0]  # Blend, transmap with alpha.
params[8] = 1.0  # Terrain.
uniform = device.create_buffer_with_data(
    data=struct.pack("<36f", *params), usage=wgpu.BufferUsage.UNIFORM | wgpu.BufferUsage.COPY_DST,
)
resources = {0: diffuse.create_view(), 1: sampler, 2: {"buffer": uniform},
             3: mask.create_view(), 11: sampler}
enhanced = device.create_buffer_with_data(data=bytes(320), usage=wgpu.BufferUsage.UNIFORM)
target = device.create_texture(size=(1, 1, 1), format="rgba8unorm",
    usage=wgpu.TextureUsage.RENDER_ATTACHMENT | wgpu.TextureUsage.COPY_SRC)
for name, alpha_block in blocks.items():
    module = device.create_shader_module(code=source + probe.replace("ALPHA_BLOCK", alpha_block))
    pipeline = device.create_render_pipeline(
        layout="auto", vertex={"module": module, "entry_point": "probe_vertex"},
        fragment={"module": module, "entry_point": "probe_fragment",
                  "targets": [{"format": "rgba8unorm"}]},
    )
    group = device.create_bind_group(layout=pipeline.get_bind_group_layout(1), entries=[
        {"binding": b, "resource": r} for b, r in resources.items()
    ])
    environment = device.create_bind_group(layout=pipeline.get_bind_group_layout(0),
        entries=[{"binding": 11, "resource": {"buffer": enhanced}}] if name == "Enhanced" else [])
    for coverage in (204, 0, 128, 255):
        device.queue.write_texture({"texture": mask}, bytes([255, 255, 255, coverage]),
            {"bytes_per_row": 4, "rows_per_image": 1}, (1, 1, 1))
        pixels = []
        for level in (0, 3):
            params[32] = level
            device.queue.write_buffer(uniform, 0, struct.pack("<36f", *params))
            encoder = device.create_command_encoder()
            render = encoder.begin_render_pass(color_attachments=[{
                "view": target.create_view(), "resolve_target": None, "load_op": "clear",
                "store_op": "store", "clear_value": (0, 0, 0, 0),
            }])
            render.set_pipeline(pipeline)
            render.set_bind_group(0, environment)
            render.set_bind_group(1, group)
            render.draw(3)
            render.end()
            device.queue.submit([encoder.finish()])
            pixels.append(tuple(device.queue.read_texture({"texture": target},
                {"bytes_per_row": 4, "rows_per_image": 1}, (1, 1, 1))))
        near, grazing = pixels
        print(f"{name} coverage {coverage}: near {near}, grazing {grazing}", flush=True)
        assert near[0] == grazing[0] and near[3] == grazing[3], "diffuse mip changed ground coverage"
        if coverage >= 204:
            assert near[0] == 0 and near[3] == 255, "covered paint leaked the lower layer"
        if coverage == 0:
            assert near == grazing == (255, 0, 0, 0), "transparent paint covered the lower layer"
print("PASS: terrain coverage stays fixed while the diffuse mip changes")
