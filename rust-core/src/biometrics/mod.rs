mod trait;
#[cfg(windows)]
mod windows;
#[cfg(android)]
mod android;

pub use trait::{FaceAuth, FaceError};
#[cfg(windows)]
pub use windows::WindowsFaceAuth;
#[cfg(android)]
pub use android::AndroidFaceAuth;

use std::sync::Arc;
use jni::JavaVM;

/// 根据平台创建人脸认证实例
pub fn create_face_auth() -> Result<Box<dyn FaceAuth>, FaceError> {
    #[cfg(windows)]
    {
        Ok(Box::new(WindowsFaceAuth::new()))
    }

    #[cfg(android)]
    {
        // Android需先获取JavaVM引用（从Flutter/Java层传入）
        let vm = match JavaVM::get_env() {
            Ok(env) => Arc::new(env.get_java_vm()?),
            Err(_) => return Err(FaceError::InitFailed("获取JavaVM失败".to_string())),
        };
        Ok(Box::new(AndroidFaceAuth::new(vm)))
    }

    #[cfg(not(any(windows, android)))]
    {
        Err(FaceError::PlatformNotSupported(
            "仅支持Windows和Android平台".to_string()
        ))
    }
}
