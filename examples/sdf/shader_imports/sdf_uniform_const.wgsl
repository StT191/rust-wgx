
// uniform
@group(0) @binding(0) var<uniform> viewport: vec3<f32>;
@group(0) @binding(1) var<uniform> scale: f32;

var<push_constant> time: f32; // time in secs


// math constants

let pi0 = 1.5707963267948966;
let pi = 3.141592653589793;
let pi2 = 6.283185307179586;
let sqrt2 = 1.4142135623730951;



// sdf functions

fn sdSphere(P: vec3<f32>, pos: vec3<f32>, r: f32) -> f32 {
  return length(P - pos) - r;
}

fn sdBox(P: vec3<f32>, pos: vec3<f32>, dim: vec3<f32>) -> f32 {
  return length(max(abs(P - pos) - dim, vec3<f32>(0.0)));
}


// operations

fn opSmoothUnion(d1: f32, d2: f32, k: f32) -> f32 {
    let h = clamp(0.5 + 0.5 * (d2-d1) / k, 0.0, 1.0);
    return mix(d2, d1, h) - k*h * (1.0-h);
}
