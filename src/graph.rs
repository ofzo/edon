use crate::compile::{compile, ModuleDependency};
use chrono::Local;
use colored::Colorize;
use queues::*;
use regex::Regex;
use relative_path::RelativePath;
use reqwest::{self};
use std::{collections::HashMap, fs};
use url::Url;

pub fn resolve(filename: &str, base: &String) -> String {
    let https = Regex::new(r#"https?://"#).unwrap();
    if https.is_match(filename) {
        return filename.to_string();
    }
    if https.is_match(&base) {
        let url = Url::parse(&base).unwrap();
        let url = url.join(&filename).unwrap();
        return url.to_string();
    }
    if filename.starts_with("/") {
        return filename.to_string();
    }
    let relative_path = RelativePath::new(filename);
    let base = RelativePath::new(base);
    let full_path = if fs::metadata(base.to_string()).unwrap().is_file() {
        base.parent().unwrap().join_normalized(relative_path)
    } else {
        base.join_normalized(relative_path)
    };
    format!("/{full_path}")
}

pub async fn load(filename: &String) -> anyhow::Result<ModuleDependency> {
    let https = Regex::new(r#"https?://"#).unwrap();
    if https.is_match(&filename) {
        let now = Local::now().timestamp_millis();
        let data = reqwest::get(filename).await?;
        let data = data.text().await?;
        let data = compile(filename, &data)?;
        println!(
            "{} {}",
            "Downland ".green(),
            format!(
                "{} cost {}ms",
                filename,
                Local::now().timestamp_millis() - now
            ),
        );
        return Ok(data);
    };
    let data = tokio::fs::read(&filename).await?;
    compile(filename, &String::from_utf8_lossy(&data).to_string())
}

#[derive(Debug)]
pub struct DependencyGraph(HashMap<String, ModuleDependency>);

impl DependencyGraph {
    pub async fn from(entry: &String, base: &String) -> anyhow::Result<Self> {
        let mut dep = DependencyGraph(HashMap::new());
        dep.append(entry, base).await?;
        Ok(dep)
    }
    pub async fn append(&mut self, source: &String, base: &String) -> anyhow::Result<()> {
        //
        let mut preload = queue![(source.clone(), base.clone())];

        let table = &mut self.0;

        while let Ok((source, base)) = preload.remove() {
            let url = resolve(&source, &base);
            let dep = load(&url).await?;
            let base = dep.filename.clone();
            for source in &dep.deps {
                if table.get(source).is_none() {
                    preload.add((source.clone(), base.clone())).unwrap();
                }
            }
            table.insert(dep.filename.clone(), dep);
        }
        Ok(())
    }
    pub fn get(&self, source: &String) -> Option<&ModuleDependency> {
        self.0.get(source)
    }
}
