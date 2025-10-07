use super::super::model::*;
use rusqlite::{params, Connection, Result as SqlResult};
use std::path::Path;
use chrono::Utc;

/// 本地数据库操作类
pub struct PersonDB {
    conn: Connection,
}

impl PersonDB {
    /// 创建/连接数据库
    pub fn new(db_path: &str) -> Self {
        // 确保目录存在（如Android的/sdcard/东方仙盟/，Windows的C:\东方仙盟\）
        let path = Path::new(db_path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }

        // 连接数据库并创建表
        let conn = Connection::open(db_path).unwrap();
        Self::create_tables(&conn).unwrap();

        PersonDB { conn }
    }

    /// 创建数据表（人员表+公司配置表）
    fn create_tables(conn: &Connection) -> SqlResult<()> {
        // 1. 人员表（按company_id隔离）
        conn.execute(
            "CREATE TABLE IF NOT EXISTS persons (
                local_id TEXT PRIMARY KEY,
                company_id TEXT NOT NULL,
                name TEXT NOT NULL,
                img_path TEXT NOT NULL,
                third_party_id TEXT NOT NULL,
                face_feature TEXT NOT NULL,
                create_time INTEGER NOT NULL,
                UNIQUE(company_id, third_party_id)
            )",
            [],
        )?;

        // 2. 公司配置表
        conn.execute(
            "CREATE TABLE IF NOT EXISTS company_configs (
                company_id TEXT PRIMARY KEY,
                third_party_api TEXT NOT NULL,
                cache_expire_seconds INTEGER NOT NULL DEFAULT 3600,
                created_at INTEGER NOT NULL
            )",
            [],
        )?;

        Ok(())
    }

    // ---------------------- 人员信息操作 ----------------------
    /// 保存人员信息
    pub fn save_person(&self, person: &PersonInfo) -> Result<(), String> {
        self.conn.execute(
            "INSERT OR REPLACE INTO persons 
             (local_id, company_id, name, img_path, third_party_id, face_feature, create_time)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                person.local_id,
                person.company_id,
                person.name,
                person.img_path,
                person.third_party_id,
                person.face_feature,
                person.create_time
            ],
        ).map_err(|e| format!("保存人员失败：{}", e))?;
        Ok(())
    }

    /// 根据公司ID查询所有人员
    pub fn get_persons_by_company(&self, company_id: &str) -> Result<Vec<PersonInfo>, String> {
        let mut stmt = self.conn.prepare(
            "SELECT local_id, company_id, name, img_path, third_party_id, face_feature, create_time
             FROM persons WHERE company_id = ?1"
        ).map_err(|e| format!("准备查询：{}", e))?;

        let person_iter = stmt.query_map([company_id], |row| {
            Ok(PersonInfo {
                local_id: row.get(0)?,
                company_id: row.get(1)?,
                name: row.get(2)?,
                img_path: row.get(3)?,
                third_party_id: row.get(4)?,
                face_feature: row.get(5)?,
                create_time: row.get(6)?,
            })
        }).map_err(|e| format!("执行查询：{}", e))?;

        let mut persons = Vec::new();
        for person in person_iter {
            persons.push(person.map_err(|e| format!("解析人员：{}", e))?);
        }
        Ok(persons)
    }

    // ---------------------- 公司配置操作 ----------------------
    /// 保存公司配置
    pub fn save_company_config(&self, config: &CompanyConfig) -> Result<(), String> {
        self.conn.execute(
            "INSERT OR REPLACE INTO company_configs 
             (company_id, third_party_api, cache_expire_seconds, created_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                config.company_id,
                config.third_party_api,
                config.cache_expire_seconds,
                config.created_at
            ],
        ).map_err(|e| format!("保存配置失败：{}", e))?;
        Ok(())
    }

    /// 根据公司ID查询配置
    pub fn get_company_config(&self, company_id: &str) -> Result<Option<CompanyConfig>, String> {
        let mut stmt = self.conn.prepare(
            "SELECT company_id, third_party_api, cache_expire_seconds, created_at
             FROM company_configs WHERE company_id = ?1"
        ).map_err(|e| format!("准备查询配置：{}", e))?;

        let config = stmt.query_row([company_id], |row| {
            Ok(CompanyConfig {
                company_id: row.get(0)?,
                third_party_api: row.get(1)?,
                cache_expire_seconds: row.get(2)?,
                created_at: row.get(3)?,
            })
        }).optional().map_err(|e| format!("查询配置：{}", e))?;

        Ok(config)
    }
}
