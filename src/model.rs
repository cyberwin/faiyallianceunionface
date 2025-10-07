// src/model.rs
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// 人员基础信息（含第三方ID）
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PersonInfo {
    pub local_id: String,       // 本地唯一ID（中间件生成）
    pub company_id: String,     // 公司ID（多公司隔离标识）
    pub name: String,           // 姓名
    pub img_path: String,       // 原始图片库路径
    pub third_party_id: String, // 第三方系统ID（如门店会员ID）
    pub face_feature: String,   // 人脸特征值（本地缓存）
}

// 识别结果（推送给第三方）
#[derive(Debug, Serialize)]
pub struct VerifyResult {
    pub company_id: String,
    pub local_id: String,
    pub third_party_id: String,
    pub name: String,
    pub success: bool,
    pub timestamp: u64, // 时间戳（毫秒）
}

// 公司配置（存储第三方API地址等）
#[derive(Debug, Serialize, Deserialize)]
pub struct CompanyConfig {
    pub company_id: String,
    pub third_party_api: String, // 第三方接收结果的API地址
    pub cache_expire: u32,       // 本地缓存过期时间（秒，默认3600）
}
