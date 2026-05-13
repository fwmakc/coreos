struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_pos: vec2<f32>,
    @location(1) center: vec2<f32>,
    @location(2) radius: f32,
    @location(3) aspect: f32,
    @location(4) color: vec4<f32>,
};

@vertex
fn vs_main(
    @location(0) pos: vec2<f32>,
    @location(1) center: vec2<f32>,
    @location(2) radius: f32,
    @location(3) aspect: f32,
    @location(4) color: vec4<f32>,
) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = vec4<f32>(pos, 0.0, 1.0);
    out.world_pos = pos;
    out.center = center;
    out.radius = radius;
    out.aspect = aspect;
    out.color = color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    if (in.radius > 0.0) {
        let delta = in.world_pos - in.center;
        let d = length(vec2<f32>(delta.x, delta.y * in.aspect));
        let edge = smoothstep(in.radius, in.radius - 0.005, d);
        if (edge == 0.0) {
            discard;
        }
        return vec4<f32>(in.color.xyz, in.color.w * edge);
    }
    return in.color;
}
