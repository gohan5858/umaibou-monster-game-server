use crate::models::Vector3;

/// 3Dベクトルの加算
pub fn add_vector3(a: &Vector3, b: &Vector3) -> Vector3 {
    Vector3 {
        x: a.x + b.x,
        y: a.y + b.y,
        z: a.z + b.z,
    }
}

/// 3Dベクトルのスカラー乗算
pub fn multiply_vector3(v: &Vector3, scalar: f32) -> Vector3 {
    Vector3 {
        x: v.x * scalar,
        y: v.y * scalar,
        z: v.z * scalar,
    }
}

/// 3Dベクトルの長さ
pub fn vector3_length(v: &Vector3) -> f32 {
    (v.x * v.x + v.y * v.y + v.z * v.z).sqrt()
}

/// 3Dベクトルの正規化
pub fn normalize_vector3(v: &Vector3) -> Vector3 {
    let length = vector3_length(v);
    if length > 0.0 {
        Vector3 {
            x: v.x / length,
            y: v.y / length,
            z: v.z / length,
        }
    } else {
        Vector3::zero()
    }
}

/// 2点間の距離
pub fn distance(a: &Vector3, b: &Vector3) -> f32 {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let dz = b.z - a.z;
    (dx * dx + dy * dy + dz * dz).sqrt()
}
