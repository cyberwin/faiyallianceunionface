use super::super::model::*;
use super::super::biometrics::{FaceAuth, FaceError, create_face_auth};
use super::super::db::PersonDB;
use reqwest::Client;
use std::sync::{Arc, Mutex, RwLock};
use std::collections::HashMap;
use chrono::Utc;
use tokio::time::{Duration, sleep};

/// 核心业务服务（线程安全）
pub struct FaceAttendanceService {
    face_auth: Arc<Mutex<dyn FaceAuth>>,       // 跨平台人脸实例
    person_db: PersonDB,                       // 本地数据库
    company_configs: Arc<RwLock<HashMap<String, CompanyConfig>>>, // 公司配置缓存
    memory_cache: Arc<Mutex<HashMap<String, PersonInfo>>>, // 内存缓存（company_id+local_id）
    http_client: Client,                       // HTTP客户端（调用第三方服务）
    db_path: String,                           // 数据库路径（跨平台适配）
}

impl FaceAttendanceService {
    /// 初始化服务
    pub fn new() -> Result<Self, FaceError> {
        // 1. 跨平台数据库路径
        let db_path = Self::get_platform_db_path();

        // 2. 创建人脸实例
        let face_auth = Arc::new(Mutex::new(create_face_auth()?));

        // 3. 初始化数据库
        let person_db = PersonDB::new(&db_path);

        // 4. 加载公司配置到内存缓存
        let company_configs = Arc::new(RwLock::new(HashMap::new()));
        Self::load_configs_to_cache(&person_db, &company_configs)?;

        Ok(Self {
            face_auth,
            person_db,
            company_configs,
            memory_cache: Arc::new(Mutex::new(HashMap::new())),
            http_client: Client::new(),
            db_path,
        })
    }

    /// 跨平台数据库路径（Windows存C盘，Android存SD卡）
    fn get_platform_db_path() -> String {
        #[cfg(windows)]
        return "C:\\东方仙盟人脸识别\\face_db.sqlite".to_string();

        #[cfg(android)]
        return "/sdcard/东方仙盟人脸识别/face_db.sqlite".to_string();

        #[cfg(not(any(windows, android)))]
        panic!("仅支持Windows和Android");
    }

    /// 加载公司配置到内存
    fn load_configs_to_cache(
        db: &PersonDB,
        cache: &Arc<RwLock<HashMap<String, CompanyConfig>>>,
    ) -> Result<(), String> {
        // 实际项目需遍历所有公司ID，此处简化为加载已存在的配置
        // （可扩展为从数据库查询所有company_id再逐个加载）
        Ok(())
    }

    // ---------------------- 对外核心接口 ----------------------
    /// 1. 添加公司配置
    pub fn add_company_config(&self, config: CompanyConfig) -> Result<(), String> {
        // 保存到数据库
        self.person_db.save_company_config(&config)?;

        // 更新内存缓存
        let mut configs = self.company_configs.write().map_err(|e| e.to_string())?;
        configs.insert(config.company_id.clone(), config);
        Ok(())
    }

    /// 2. 从图片路径注册人员
    pub fn register_from_img(&self, req: RegisterReq) -> Result<PersonInfo, String> {
        // 校验公司配置是否存在
        let config = {
            let configs = self.company_configs.read().map_err(|e| e.to_string())?;
            configs.get(&req.company_id)
                .cloned()
                .ok_or_else(|| format!("公司{}未配置", req.company_id))?
        };

        // 提取人脸特征
        let face_feature = {
            let mut face_auth = self.face_auth.lock().map_err(|e| e.to_string())?;
            face_auth.extract_feature_from_path(&req.img_path)
                .map_err(|e| format!("提取特征失败：{}", e))?
        };

        // 生成本地ID（公司ID+时间戳+随机数）
        let local_id = format!(
            "{}_{}_{}",
            req.company_id,
            Utc::now().timestamp_millis(),
            rand::Rng::gen_range(&mut rand::thread_rng(), 1000..9999)
        );

        // 构造人员信息
        let person = PersonInfo {
                        local_id: local_id.clone(),
            company_id: req.company_id.clone(),
            name: req.name,
            img_path: req.img_path,
            third_party_id: req.third_party_id,
            face_feature,
            create_time: Utc::now().timestamp_millis(),
        };

        // 保存到数据库和内存缓存
        self.person_db.save_person(&person)?;
        let mut memory_cache = self.memory_cache.lock().map_err(|e| e.to_string())?;
        memory_cache.insert(format!("{}_{}", req.company_id, local_id), person.clone());

        Ok(person)
    }

    /// 3. 人脸比对+推送第三方+接收闸机指令
    pub async fn verify_and_notify(&self, company_id: &str) -> Result<ThirdPartyResp, String> {
        // 步骤1：校验公司配置
        let config = {
            let configs = self.company_configs.read().map_err(|e| e.to_string())?;
            configs.get(company_id)
                .cloned()
                .ok_or_else(|| format!("公司{}未配置", company_id))?
        };

        // 步骤2：实时捕获人脸特征
        let live_feat = {
            let mut face_auth = self.face_auth.lock().map_err(|e| e.to_string())?;
            face_auth.capture_live_feature()
                .map_err(|e| format!("捕获人脸失败：{}", e))?
        };

        // 步骤3：比对（优先内存缓存→数据库）
        let matched_person = self.match_face(company_id, &live_feat)?;
        if matched_person.is_none() {
            return Ok(ThirdPartyResp {
                status: 1,
                message: "未匹配到白名单人员".to_string(),
                request_id: gen_request_id(),
            });
        }
        let person = matched_person.unwrap();

        // 步骤4：推送比对结果到第三方服务器
        let request_id = gen_request_id();
        let push_req = VerifyPushReq {
            company_id: company_id.to_string(),
            local_id: person.local_id.clone(),
            third_party_id: person.third_party_id.clone(),
            name: person.name.clone(),
            success: true,
            timestamp: Utc::now().timestamp_millis(),
            request_id: request_id.clone(),
        };

        // 调用第三方API并等待回调（超时5秒）
        let third_resp = tokio::time::timeout(
            Duration::from_secs(5),
            self.call_third_party(&config.third_party_api, &push_req)
        ).await
            .map_err(|e| format!("第三方请求超时：{}", e))?
            .map_err(|e| format!("第三方调用失败：{}", e))?;

        // 步骤5：返回闸机指令（status=9成功）
        Ok(ThirdPartyResp {
            status: third_resp.status,
            message: third_resp.message,
            request_id: third_resp.request_id,
        })
    }

    // ---------------------- 辅助方法 ----------------------
    /// 人脸比对逻辑（内存缓存→数据库）
    fn match_face(&self, company_id: &str, live_feat: &str) -> Result<Option<PersonInfo>, String> {
        // 1. 查内存缓存（前缀：company_id_）
        let memory_cache = self.memory_cache.lock().map_err(|e| e.to_string())?;
        let cache_key_prefix = format!("{}_", company_id);
        for (key, person) in memory_cache.iter() {
            if key.starts_with(&cache_key_prefix) {
                let similarity = {
                    let face_auth = self.face_auth.lock().map_err(|e| e.to_string())?;
                    face_auth.calculate_similarity(live_feat, &person.face_feature)
                        .map_err(|e| format!("计算相似度失败：{}", e))?
                };
                if similarity >= 0.6 {
                    return Ok(Some(person.clone()));
                }
            }
        }
        drop(memory_cache); // 释放锁

        // 2. 查数据库
        let persons = self.person_db.get_persons_by_company(company_id)?;
        for person in persons {
            let similarity = {
                let face_auth = self.face_auth.lock().map_err(|e| e.to_string())?;
                face_auth.calculate_similarity(live_feat, &person.face_feature)
                    .map_err(|e| format!("计算相似度失败：{}", e))?
            };
            if similarity >= 0.6 {
                // 更新到内存缓存
                let mut memory_cache = self.memory_cache.lock().map_err(|e| e.to_string())?;
                memory_cache.insert(format!("{}_{}", company_id, person.local_id), person.clone());
                return Ok(Some(person));
            }
        }

        Ok(None)
    }

    /// 调用第三方服务器API
    async fn call_third_party(
        &self,
        third_api: &str,
        push_req: &VerifyPushReq
    ) -> Result<ThirdPartyResp, String> {
        let resp = self.http_client.post(third_api)
            .header("Content-Type", "application/json")
            .json(push_req)
            .send()
            .await
            .map_err(|e| format!("HTTP请求失败：{}", e))?;

        if !resp.status().is_success() {
            return Err(format!("第三方返回状态码：{}", resp.status()));
        }

        resp.json::<ThirdPartyResp>()
            .await
            .map_err(|e| format!("解析第三方响应失败：{}", e))
    }
}
