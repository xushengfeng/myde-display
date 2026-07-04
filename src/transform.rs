use nalgebra::{Matrix3, Vector3};

pub struct TransformManager;

impl TransformManager {
    pub fn new() -> Self {
        TransformManager
    }

    pub fn create_transform(
        &self,
        rotation: f64,
        scale_x: f64,
        scale_y: f64,
        translate_x: f64,
        translate_y: f64,
        origin_x: f64,
        origin_y: f64,
    ) -> Transform {
        // 创建变换矩阵
        // 首先平移到原点
        let to_origin = Matrix3::new(
            1.0, 0.0, -origin_x,
            0.0, 1.0, -origin_y,
            0.0, 0.0, 1.0,
        );

        // 旋转
        let cos_theta = rotation.cos();
        let sin_theta = rotation.sin();
        let rotation_matrix = Matrix3::new(
            cos_theta, -sin_theta, 0.0,
            sin_theta, cos_theta, 0.0,
            0.0, 0.0, 1.0,
        );

        // 缩放
        let scale_matrix = Matrix3::new(
            scale_x, 0.0, 0.0,
            0.0, scale_y, 0.0,
            0.0, 0.0, 1.0,
        );

        // 平移回来
        let from_origin = Matrix3::new(
            1.0, 0.0, origin_x,
            0.0, 1.0, origin_y,
            0.0, 0.0, 1.0,
        );

        // 最终平移
        let translation_matrix = Matrix3::new(
            1.0, 0.0, translate_x,
            0.0, 1.0, translate_y,
            0.0, 0.0, 1.0,
        );

        // 组合变换：T * F * S * R * T_origin
        let result = translation_matrix * from_origin * scale_matrix * rotation_matrix * to_origin;

        Transform {
            matrix: [
                result[(0, 0)], result[(0, 1)], result[(0, 2)],
                result[(1, 0)], result[(1, 1)], result[(1, 2)],
                result[(2, 0)], result[(2, 1)], result[(2, 2)],
            ],
        }
    }

    pub fn apply_transform(&self, matrix: &[f64; 9], x: f64, y: f64) -> (f64, f64) {
        let transform_matrix = Matrix3::new(
            matrix[0], matrix[1], matrix[2],
            matrix[3], matrix[4], matrix[5],
            matrix[6], matrix[7], matrix[8],
        );

        let point = Vector3::new(x, y, 1.0);
        let result = transform_matrix * point;

        let w = result[2];
        if w.abs() < f64::EPSILON {
            return (0.0, 0.0);
        }

        (result[0] / w, result[1] / w)
    }

    pub fn compose_transforms(&self, matrices: &[[f64; 9]]) -> [f64; 9] {
        if matrices.is_empty() {
            return [
                1.0, 0.0, 0.0,
                0.0, 1.0, 0.0,
                0.0, 0.0, 1.0,
            ];
        }

        let mut result = Matrix3::new(
            matrices[0][0], matrices[0][1], matrices[0][2],
            matrices[0][3], matrices[0][4], matrices[0][5],
            matrices[0][6], matrices[0][7], matrices[0][8],
        );

        for matrix in matrices.iter().skip(1) {
            let next = Matrix3::new(
                matrix[0], matrix[1], matrix[2],
                matrix[3], matrix[4], matrix[5],
                matrix[6], matrix[7], matrix[8],
            );
            result = next * result;
        }

        [
            result[(0, 0)], result[(0, 1)], result[(0, 2)],
            result[(1, 0)], result[(1, 1)], result[(1, 2)],
            result[(2, 0)], result[(2, 1)], result[(2, 2)],
        ]
    }

    pub fn create_rotation(&self, angle: f64, origin_x: f64, origin_y: f64) -> Transform {
        self.create_transform(angle, 1.0, 1.0, 0.0, 0.0, origin_x, origin_y)
    }

    pub fn create_scale(&self, scale_x: f64, scale_y: f64, origin_x: f64, origin_y: f64) -> Transform {
        self.create_transform(0.0, scale_x, scale_y, 0.0, 0.0, origin_x, origin_y)
    }

    pub fn create_translation(&self, x: f64, y: f64) -> Transform {
        self.create_transform(0.0, 1.0, 1.0, x, y, 0.0, 0.0)
    }

    pub fn invert_transform(matrix: &[f64; 9]) -> Option<[f64; 9]> {
        let m = Matrix3::new(
            matrix[0], matrix[1], matrix[2],
            matrix[3], matrix[4], matrix[5],
            matrix[6], matrix[7], matrix[8],
        );

        m.try_inverse().map(|inv| {
            [
                inv[(0, 0)], inv[(0, 1)], inv[(0, 2)],
                inv[(1, 0)], inv[(1, 1)], inv[(1, 2)],
                inv[(2, 0)], inv[(2, 1)], inv[(2, 2)],
            ]
        })
    }
}

pub struct Transform {
    pub matrix: [f64; 9],
}