// Perlin noise implementation
// Based on https://github.com/mgord9518/perlin-zig/blob/main/lib/perlin.zig

const PERMUTATION: [u8; 256] = [
    151, 160, 137, 91,  90,  15,  131, 13,  201, 95,  96,  53,  194, 233, 7,   225,
    140, 36,  103, 30,  69,  142, 8,   99,  37,  240, 21,  10,  23,  190, 6,   148,
    247, 120, 234, 75,  0,   26,  197, 62,  94,  252, 219, 203, 117, 35,  11,  32,
    57,  177, 33,  88,  237, 149, 56,  87,  174, 20,  125, 136, 171, 168, 68,  175,
    74,  165, 71,  134, 139, 48,  27,  166, 77,  146, 158, 231, 83,  111, 229, 122,
    60,  211, 133, 230, 220, 105, 92,  41,  55,  46,  245, 40,  244, 102, 143, 54,
    65,  25,  63,  161, 1,   216, 80,  73,  209, 76,  132, 187, 208, 89,  18,  169,
    200, 196, 135, 130, 116, 188, 159, 86,  164, 100, 109, 198, 173, 186, 3,   64,
    52,  217, 226, 250, 124, 123, 5,   202, 38,  147, 118, 126, 255, 82,  85,  212,
    207, 206, 59,  227, 47,  16,  58,  17,  182, 189, 28,  42,  223, 183, 170, 213,
    119, 248, 152, 2,   44,  154, 163, 70,  221, 153, 101, 155, 167, 43,  172, 9,
    129, 22,  39,  253, 19,  98,  108, 110, 79,  113, 224, 232, 178, 185, 112, 104,
    218, 246, 97,  228, 251, 34,  242, 193, 238, 210, 144, 12,  191, 179, 162, 241,
    81,  51,  145, 235, 249, 14,  239, 107, 49,  192, 214, 31,  181, 199, 106, 157,
    184, 84,  204, 176, 115, 121, 50,  45,  127, 4,   150, 254, 138, 236, 205, 93,
    222, 114, 67,  29,  24,  72,  243, 141, 128, 195, 78,  66,  215, 61,  156, 180,
];

pub fn noise3d(x: f64, y: f64, z: f64) -> f64 {
    let x_floor = x.floor();
    let y_floor = y.floor();
    let z_floor = z.floor();
    
    let x_int = (x_floor as i32 & 255) as u8;
    let y_int = (y_floor as i32 & 255) as u8;
    let z_int = (z_floor as i32 & 255) as u8;
    
    let x_frac = x - x_floor;
    let y_frac = y - y_floor;
    let z_frac = z - z_floor;
    
    let u = fade(x_frac);
    let v = fade(y_frac);
    let w = fade(z_frac);
    
    // Hash coordinates of the 8 cube corners
    let a = PERMUTATION[x_int as usize];
    let aa = PERMUTATION[(a.wrapping_add(y_int)) as usize];
    let ab = PERMUTATION[(a.wrapping_add(y_int).wrapping_add(1)) as usize];
    let b = PERMUTATION[(x_int.wrapping_add(1)) as usize];
    let ba = PERMUTATION[(b.wrapping_add(y_int)) as usize];
    let bb = PERMUTATION[(b.wrapping_add(y_int).wrapping_add(1)) as usize];
    
    // Add blended results from all 8 corners of the cube
    lerp(w,
        lerp(v,
            lerp(u, grad3d(PERMUTATION[(aa.wrapping_add(z_int)) as usize], x_frac, y_frac, z_frac),
                    grad3d(PERMUTATION[(ba.wrapping_add(z_int)) as usize], x_frac - 1.0, y_frac, z_frac)),
            lerp(u, grad3d(PERMUTATION[(ab.wrapping_add(z_int)) as usize], x_frac, y_frac - 1.0, z_frac),
                    grad3d(PERMUTATION[(bb.wrapping_add(z_int)) as usize], x_frac - 1.0, y_frac - 1.0, z_frac))),
        lerp(v,
            lerp(u, grad3d(PERMUTATION[(aa.wrapping_add(z_int).wrapping_add(1)) as usize], x_frac, y_frac, z_frac - 1.0),
                    grad3d(PERMUTATION[(ba.wrapping_add(z_int).wrapping_add(1)) as usize], x_frac - 1.0, y_frac, z_frac - 1.0)),
            lerp(u, grad3d(PERMUTATION[(ab.wrapping_add(z_int).wrapping_add(1)) as usize], x_frac, y_frac - 1.0, z_frac - 1.0),
                    grad3d(PERMUTATION[(bb.wrapping_add(z_int).wrapping_add(1)) as usize], x_frac - 1.0, y_frac - 1.0, z_frac - 1.0))))
}

fn fade(t: f64) -> f64 {
    t * t * t * (t * (6.0 * t - 15.0) + 10.0)
}

fn lerp(t: f64, a: f64, b: f64) -> f64 {
    a + t * (b - a)
}

fn grad3d(h: u8, x: f64, y: f64, z: f64) -> f64 {
    match h & 15 {
        0 | 12 => x + y,
        1 | 14 => y - x,
        2 => x - y,
        3 => -x - y,
        4 => x + z,
        5 => z - x,
        6 => x - z,
        7 => -x - z,
        8 => y + z,
        9 | 13 => z - y,
        10 => y - z,
        11 | 15 => -y - z,
        _ => 0.0, // Should never happen
    }
}