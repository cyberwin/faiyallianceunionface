use axum::Server;
use std::net::SocketAddr;
use log::{info, warn};
use env_logger::Env;

mod model;
mod biometrics;
mod db;
mod service;
mod api;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. 初始化日志（输出到控制台）
    env_logger::Builder::from_env(Env::default().default_filter_or("info"))
        .format_timestamp_millis()
        .init();

    info!("=== 东方仙盟人脸识别接口中心 启动 ===");

    // 2. 初始化核心服务
    let service = match service::FaceAttendanceService::new() {
        Ok(s) => Arc::new(s),
        Err(e) => {
            warn!("服务初始化失败：{}", e);
            return Err(Box::new(e));
        }
    };

    // 3. 构建API路由
    let app = api::build_router(service.clone());

    // 4. 启动HTTP服务器（监听0.0.0.0:8080，支持局域网访问）
    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    info!("API服务器启动：http://{}", addr);

    Server::bind(&addr)
        .serve(app.into_make_svc())
        .await
        .map_err(|e| {
            warn!("服务器启动失败：{}", e);
            e.into()
        })?;

    Ok(())
}
