use crate::AppResult;
use crate::models::{Email, Tag};
use regex::Regex;
use std::collections::HashMap;

lazy_static::lazy_static! {
    static ref WORK_DOMAINS: Regex = Regex::new(r"@(work|company|corp|business)\.[a-z]+$").unwrap();
    static ref SUBSCRIPTION_KEYWORDS: Regex = Regex::new(r"(?i)(newsletter|subscription|unsubscribe|订|订阅|news|资讯|推广|营销|广告)").unwrap();
    static ref SPAM_KEYWORDS: Regex = Regex::new(r"(?i)(spam|junk|垃圾|中奖|免费|赚钱|优惠|折扣|促销|中奖|骗局|钓鱼|verify|确认|urgent|紧急)").unwrap();
    static ref WORK_KEYWORDS: Regex = Regex::new(r"(?i)(meeting|会议|report|报告|deadline|截止|project|项目|client|客户|同事|@工作|工作邮件)").unwrap();
    static ref PERSONAL_DOMAINS: Regex = Regex::new(r"@(gmail|qq|163|outlook|hotmail|yahoo|sina|foxmail)\.[a-z]+$").unwrap();
}

pub struct EmailClassifier {
    system_tags: HashMap<String, i64>,
}

impl EmailClassifier {
    pub fn new(tags: &[Tag]) -> Self {
        let mut system_tags = HashMap::new();
        for tag in tags {
            if tag.is_system {
                system_tags.insert(tag.name.clone(), tag.id);
            }
        }
        Self { system_tags }
    }

    pub fn classify(&self, email: &Email) -> Vec<i64> {
        let mut tags = Vec::new();

        let domain = self.extract_domain(&email.sender_email);
        let subject = &email.subject;
        let body = &email.body_text;

        let combined = format!("{} {}", subject, body);

        if self.is_spam(&combined, &domain) {
            if let Some(tag_id) = self.system_tags.get("垃圾") {
                tags.push(*tag_id);
                return tags;
            }
        }

        if self.is_subscription(&combined, &domain) {
            if let Some(tag_id) = self.system_tags.get("订阅") {
                tags.push(*tag_id);
                return tags;
            }
        }

        if self.is_work(&combined, &domain) {
            if let Some(tag_id) = self.system_tags.get("工作") {
                tags.push(*tag_id);
                return tags;
            }
        }

        if self.is_personal(&domain) {
            if let Some(tag_id) = self.system_tags.get("个人") {
                tags.push(*tag_id);
            }
        }

        tags
    }

    fn extract_domain(&self, email: &str) -> String {
        email
            .find('@')
            .map(|idx| email[idx + 1..].to_lowercase())
            .unwrap_or_default()
    }

    fn is_spam(&self, content: &str, domain: &str) -> bool {
        if SPAM_KEYWORDS.is_match(content) {
            return true;
        }
        let spam_domains = ["spam", "junk", "unknown"];
        spam_domains.iter().any(|d| domain.contains(d))
    }

    fn is_subscription(&self, content: &str, domain: &str) -> bool {
        if SUBSCRIPTION_KEYWORDS.is_match(content) {
            return true;
        }
        let sub_domains = ["newsletter", "mailgun", "sendgrid", "mailchimp"];
        sub_domains.iter().any(|d| domain.contains(d))
    }

    fn is_work(&self, content: &str, domain: &str) -> bool {
        if WORK_DOMAINS.is_match(&format!("@{}", domain)) {
            return true;
        }
        if WORK_KEYWORDS.is_match(content) {
            return true;
        }
        false
    }

    fn is_personal(&self, domain: &str) -> bool {
        PERSONAL_DOMAINS.is_match(&format!("@{}", domain))
    }
}

pub fn auto_classify_email(email: &Email, tags: &[Tag]) -> Vec<i64> {
    let classifier = EmailClassifier::new(tags);
    classifier.classify(email)
}
