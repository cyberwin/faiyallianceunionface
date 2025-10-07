// src/service/face_service.rs
use super::super::{model::*, db::*, http::*};
use crate::biometrics::FaceAuth;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

// 全局服务（带多线程安全锁）
pub struct FaceAttendanceService {
    face_auth: Arc<Mutex<dyn FaceAuth>>, // 跨平台人脸认证实例
    person_db: PersonDB,                 // 本地人员数据库（按公司隔离）
    company_config: Arc<Mutex<HashMap<String, CompanyConfig>>>, // 公司配置缓存
    memory_cache: Arc<Mutex<HashMap<String, PersonInfo>>>, // 内存缓存（company_id+local_id为key）
}

impl FaceAttendanceService {
    // 初始化服务
    pub fn new(face_auth: Box<dyn FaceAuth>) -> Self {
        Self {
            face_auth: Arc::new(Mutex::new(face_auth)),
            person_db: PersonDB::new("./data/person_db.sqlite"), // 本地SQLite数据库
            company_config: Arc::new(Mutex::new(HashMap::new())),
            memory_cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    // 1. 从指定图片库抽图，生成人员信息并缓存
    pub fn register_from_img_path(&self, req: RegisterReq) -> Result<PersonInfo, String> {
        // 校验公司配置是否存在
        let company_cfg = self.get_company_config(&req.company_id)?;
        // 从图片路径加载图片（假设图片库可通过路径访问）
        let img = image::open(&req.img_path).map_err(|e| format!("图片加载失败：{}", e))?;
        // 提取人脸特征
        let face_feature = {
            let mut face_auth = self.face_auth.lock().map_err(|e| e.to_string())?;
            face_auth.extract_feature_from_img(&img)?
        };
        // 生成本地ID（公司ID+时间戳+随机数）
        let local_id = format!(
            "{}_{}_{}",
            req.company_id,
            SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis(),
            rand::random::<u32>() % 10000
        );
        // 构造人员信息
        let person = PersonInfo {
            local_id: local_id.clone(),
            company_id: req.company_id.clone(),
            name: req.name,
            img_path: req.img_path,
            third_party_id: req.third_party_id,
            face_feature: face_feature.clone(),
        };
        // 存入本地数据库
        self.person_db.save_person(&person)?;
        // 存入内存缓存（提速后续比对）
        self.memory_cache.lock().unwrap().insert(
            format!("{}_{}", req.company_id, local_id),
            person.clone()
        );
        Ok(person)
    }

    // 2. 人脸比对（优先查内存缓存→本地数据库→不查原始图片库）
    pub async fn verify_face(&self, company_id: &str) -> Result<VerifyResult, String> {
        // 1. 捕获实时人脸特征
        let live_feature = {
            let mut face_auth = self.face_auth.lock().map_err(|e| e.to_string())?;
            face_auth.capture_and_extract_feature()?
        };
        // 2. 优先从内存缓存比对（最快）
        let mut memory_cache = self.memory_cache.lock().unwrap();
        let cache_key_prefix = format!("{}_", company_id);
        for (key, person) in memory_cache.iter() {
            if key.starts_with(&cache_key_prefix) {
                let similarity = {
                    let face_auth = self.face_auth.lock().map_err(|e| e.to_string())?;
                    face_auth.calculate_similarity(&live_feature, &person.face_feature)?
                };
                if similarity >= 0.6 {
                    // 比对成功，生成结果并推送给第三方
                    let result = VerifyResult {
                        company_id: company_id.to_string(),
                        local_id: person.local_id.clone(),
                        third_party_id: person.third_party_id.clone(),
                        name: person.name.clone(),
                        success: true,
                        timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64,
                    };
                    self.push_to_third_party(&company_id, &result).await?;
                    return Ok(result);
                }
            }
        }
        // 3. 内存缓存未命中，查本地数据库
        let company_persons = self.person_db.get_persons_by_company(company_id)?;
        for person in company_persons {
            let similarity = {
                let face_auth = self.face_auth.lock().map_err(|e| e.to_string())?;
                face_auth.calculate_similarity(&live_feature, &person.face_feature)?
            };
            if similarity >= 0.6 {
                // 更新到内存缓存（下次更快）
                memory_cache.insert(format!("{}_{}", company_id, person.local_id), person.clone());
                let result = VerifyResult {
                    company_id: company_id.to_string(),
                    local_id: person.local_id.clone(),
                    third_party_id: person.third_party_id.clone(),
                    name: person.name.clone(),
                    success: true,
                    timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64,
                };
                self.push_to_third_party(&company_id, &result).await?;
                return Ok(result);
            }
        }
        // 4. 比对失败
        let result = VerifyResult {
            company_id: company_id.to_string(),
            local_id: "".to_string(),
            third_party_id: "".to_string(),
            name: "".to_string(),
            success: false,
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64,
        };
        Ok(result)
    }

    // 3. 推送结果到第三方服务
    async fn push_to_third_party(&self, company_id: &str, result: &VerifyResult) -> Result<(), String> {
        let company_cfg = self.get_company_config(company_id)?;
        // 发送POST请求（JSON格式）
        let client = reqwest::Client::new();
        let resp = client.post(&company_cfg.third_party_api)
            .json(result)
            .send()
            .await
            .map_err(|e| format!("第三方请求失败：{}", e))?;
        if !resp.status().is_success() {
            return Err(format!("第三方返回错误：{}", resp.status()));
        }
        Ok(())
    }

    // 辅助：获取公司配置
    fn get_company_config(&self, company_id: &str) -> Result<CompanyConfig, String> {
        let configs = self.company_config.lock().unwrap();
        configs.get(company_id)
            .cloned()
            .ok_or_else(|| format!("公司{}未配置第三方API地址", company_id))
    }

    // 辅助：添加公司配置
    pub fn add_company_config(&self, config: CompanyConfig) {
        let mut configs = self.company_config.lock().unwrap();
        configs.insert(config.company_id.clone(), config);
    }
}

// 注册请求参数
#[derive(Debug, Deserialize)]
pub struct RegisterReq {
    pub company_id: String,
    pub name: String,
    pub img_path: String,
    pub third_party_id: String,
}
