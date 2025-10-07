// src/api/mod.rs
use axum::{Router, routing::post, Json, extract::Path};
use super::super::service::FaceAttendanceService;
use super::super::model::*;
use std::sync::Arc;

// 构建API路由
pub fn build_router(service: Arc<FaceAttendanceService>) -> Router {
    Router::new()
        // 1. 添加公司配置（仅管理员调用）
        .route("/config/company", post(add_company_config))
        // 2. 从图片路径注册人员
        .route("/register/from-img", post(register_from_img))
        // 3. 人脸比对（带公司ID）
        .route("/verify/:company_id", post(verify_face))
        .with_state(service)
}

// 添加公司配置接口
async fn add_company_config(
    Json(config): Json<CompanyConfig>,
    service: Arc<FaceAttendanceService>,
) -> Json<Result<String, String>> {
    service.add_company_config(config.clone());
    Json(Ok(format!("公司{}配置添加成功", config.company_id)))
}

// 从图片路径注册接口
async fn register_from_img(
    Json(req): Json<RegisterReq>,
    service: Arc<FaceAttendanceService>,
) -> Json<Result<PersonInfo, String>> {
    Json(service.register_from_img_path(req))
}

// 人脸比对接口（路径参数带公司ID）
async fn verify_face(
    Path(company_id): Path<String>,
    service: Arc<FaceAttendanceService>,
) -> Json<Result<VerifyResult, String>> {
    Json(service.verify_face(&company_id).await)
}
