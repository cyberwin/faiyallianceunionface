use axum::{Router, routing::{post, get}, Json, extract::{Path, State}, http::StatusCode};
use super::super::model::*;
use super::super::service::FaceAttendanceService;
use super::super::biometrics::FaceError;
use std::sync::Arc;

/// 构建API路由
pub fn build_router(service: Arc<FaceAttendanceService>) -> Router {
    Router::new()
        // 1. 健康检查（测试服务是否启动）
        .route("/health", get(health_check))
        // 2. 添加公司配置（仅管理员调用）
        .route("/config/company", post(add_company_config))
        // 3. 从图片路径注册人员
        .route("/register", post(register_person))
        // 4. 人脸比对+闸机指令（核心接口）
        .route("/verify/:company_id", post(verify_face))
        .with_state(service)
}

// ---------------------- API接口实现 ----------------------
/// 健康检查
async fn health_check() -> (StatusCode, &'static str) {
    (StatusCode::OK, "东方仙盟人脸识别接口中心 - 服务正常")
}

/// 添加公司配置
async fn add_company_config(
    State(service): State<Arc<FaceAttendanceService>>,
    Json(config): Json<CompanyConfig>,
) -> Json<ApiResp<()>> {
    match service.add_company_config(config) {
        Ok(_) => Json(ApiResp::Success {
            data: (),
            message: "公司配置添加成功",
        }),
        Err(e) => Json(ApiResp::Error {
            code: 1001,
            message: e,
        }),
    }
}

/// 注册人员（从图片路径）
async fn register_person(
    State(service): State<Arc<FaceAttendanceService>>,
    Json(req): Json<RegisterReq>,
) -> Json<ApiResp<PersonInfo>> {
    match service.register_from_img(req) {
        Ok(person) => Json(ApiResp::Success {
            data: person,
            message: "人员注册成功",
        }),
        Err(e) => Json(ApiResp::Error {
            code: 1002,
            message: e,
        }),
    }
}

/// 人脸比对+闸机指令
async fn verify_face(
    State(service): State<Arc<FaceAttendanceService>>,
    Path(company_id): Path<String>,
) -> Json<ApiResp<ThirdPartyResp>> {
    match service.verify_and_notify(&company_id).await {
        Ok(resp) => {
            let message = if resp.status == 9 {
                "闸机允许开门"
            } else {
                "闸机拒绝开门"
            };
            Json(ApiResp::Success {
                data: resp,
                message,
            })
        }
        Err(e) => Json(ApiResp::Error {
            code: 1003,
            message: e,
        }),
    }
}
