use crate::compile::{compile, ModuleDependency};
use anyhow::anyhow;
use chrono::Local;
use colored::Colorize;
use queues::*;
use regex::Regex;
use relative_path::RelativePath;
use reqwest::{self};
use std::{
    collections::HashMap,
    io::{Error, ErrorKind},
};
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
    let relative_path = RelativePath::new(filename);
    let full_path = relative_path.to_logical_path(&base);
    return full_path.to_string_lossy().to_string();
}

pub async fn load(filename: &String) -> anyhow::Result<ModuleDependency> {
    let https = Regex::new(r#"https?://"#).unwrap();
    if https.is_match(&filename) {
        let now = Local::now().timestamp_millis();
        let data = reqwest::get(filename).await?;
        let data = data.text().await?;
        let data = compile(filename, &data);
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
    let data = compile(filename, &String::from_utf8_lossy(&data).to_string());
    return Ok(data);
}

#[derive(Debug)]
pub struct DependencyGraph(HashMap<String, ModuleDependency>);

impl DependencyGraph {
    pub async fn from(entry: &String, base: &String) -> anyhow::Result<Self> {
        let mut dep = DependencyGraph(HashMap::new());
        dep.append(entry, base).await?;
        Ok(dep)
    }
    pub async fn append(&mut self, entry: &String, base: &String) -> anyhow::Result<()> {
        //
        let mut preload = queue![(entry.clone(), base.clone())];

        let table = &mut self.0;

        while let Ok((entry, base)) = preload.remove() {
            let url = resolve(&entry, &base);
            let dep = load(&url).await?;
            let base = dep.filename.clone();
            for specifier in &dep.deps {
                if table.get(specifier).is_none() {
                    preload.add((specifier.clone(), base.clone())).unwrap();
                }
            }
            table.insert(dep.filename.clone(), dep);
        }
        Ok(())
    }
    pub fn get(&self, specifier: &String) -> Option<&ModuleDependency> {
        self.0.get(specifier)
    }
}
