use super::trait::*;
use jni::{JavaVM, JNIEnv, objects::{JClass, JObject}};
use robius_authentication::android::AndroidBiometrics;
use face_recognition_rs::{FaceRecognizer, FaceEncoding};
use image::DynamicImage;
use std::sync::Arc;

/// Android平台人脸实现
pub struct AndroidFaceAuth {
    vm: Arc<JavaVM>,             // Android Java虚拟机引用
    biometrics: AndroidBiometrics, // 安卓生物识别（指纹/人脸）
    recognizer: FaceRecognizer,   // 人脸检测/特征提取
    camera_index: usize,          // 摄像头索引（默认0=后置，1=前置）
}

impl AndroidFaceAuth {
    pub fn new(vm: Arc<JavaVM>) -> Self {
        // 获取JNI环境
        let env = vm.attach_current_thread()
            .map_err(|e| FaceError::InitFailed(format!("获取JNI环境：{}", e)))
            .unwrap();

        Self {
            vm: vm.clone(),
            biometrics: AndroidBiometrics::new(&env, "东方仙盟人脸识别"),
            recognizer: FaceRecognizer::new(0.6),
            camera_index: 1, // 考勤场景优先用前置摄像头
        }
    }

    // 辅助：获取当前JNI环境
    fn get_env(&self) -> Result<JNIEnv, FaceError> {
        self.vm.attach_current_thread()
            .map_err(|e| FaceError::Other(format!("Attach线程失败：{}", e)))
    }

    // 辅助：图片格式转换
    fn img_to_encoding(&self, img: &DynamicImage) -> Result<FaceEncoding, FaceError> {
        let (width, height) = img.dimensions();
        let rgb8 = img.to_rgb8();
        let pixels: Vec<u8> = rgb8.into_raw();

        FaceEncoding::from_rgb_pixels(width as i32, height as i32, &pixels)
            .map_err(|e| FaceError::FeatureExtractFailed(format!("格式转换：{}", e)))
    }
}

impl FaceAuth for AndroidFaceAuth {
    fn init(&mut self) -> Result<(), FaceError> {
        let env = self.get_env()?;

        // 1. 请求摄像头+生物识别权限
        self.biometrics.request_permissions(&env)
            .map_err(|e| FaceError::InitFailed(format!("权限申请：{}", e)))?;
        
        // 2. 初始化摄像头（Android需动态申请权限后启动）
        self.recognizer.init_camera(self.camera_index)
            .map_err(|e| FaceError::CameraError(format!("摄像头启动：{}", e)))?;
        
        Ok(())
    }

    fn extract_feature_from_image(&mut self, img: &DynamicImage) -> Result<String, FaceError> {
        // 同Windows逻辑：检测人脸→提取特征→序列化
        let encoding = self.img_to_encoding(img)?;
        let face_encodings = self.recognizer.get_face_encodings(&encoding)
            .map_err(|e| FaceError::FeatureExtractFailed(format!("提取特征：{}", e)))?;
        
        if face_encodings.is_empty() {
            return Err(FaceError::NoFaceDetected);
        }

        let feat_str = serde_json::to_string(&face_encodings[0])
            .map_err(|e| FaceError::Other(format!("特征序列化：{}", e)))?;
        
        Ok(feat_str)
    }

    fn capture_live_feature(&mut self) -> Result<String, FaceError> {
        // 实时捕获前置摄像头画面
        let frame = self.recognizer.capture_frame()
            .map_err(|e| FaceError::CameraError(format!("捕获画面：{}", e)))?;
        
        let encoding = FaceEncoding::from_bgr_frame(&frame)
            .map_err(|e| FaceError::ImageError(format!("帧转换：{}", e)))?;
        
        let face_encodings = self.recognizer.get_face_encodings(&encoding)
            .map_err(|e| FaceError::FeatureExtractFailed(format!("实时特征提取：{}", e)))?;
        
        if face_encodings.is_empty() {
            return Err(FaceError::NoFaceDetected);
        }

        let feat_str = serde_json::to_string(&face_encodings[0])
            .map_err(|e| FaceError::Other(format!("实时特征序列化：{}", e)))?;
        
        Ok(feat_str)
    }

    fn calculate_similarity(&self, feat1: &str, feat2: &str) -> Result<f32, FaceError> {
        // 与Windows完全一致的相似度计算逻辑
        let feat1: Vec<f32> = serde_json::from_str(feat1)
            .map_err(|e| FaceError::Other(format!("特征1解析：{}", e)))?;
        let feat2: Vec<f32> = serde_json::from_str(feat2)
            .map_err(|e| FaceError::Other(format!("特征2解析：{}", e)))?;
        
        let distance = self.recognizer.calculate_distance(&feat1, &feat2)
            .map_err(|e| FaceError::Other(format!("计算距离：{}", e)))?;
        let similarity = 1.0 - (distance / 1.2);
        
        Ok(similarity.max(0.0).min(1.0))
    }
}
