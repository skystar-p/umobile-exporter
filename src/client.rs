use reqwest::header::USER_AGENT;
use scraper::{Html, Selector};
use thiserror::Error;

pub struct UmobileClient {
    client: reqwest::Client,
}

#[derive(Error, Debug)]
pub enum ClientError {
    #[error("login failed")]
    LoginError,
    #[error("usage fetch error")]
    UsageFetchError,
    #[error("bill fetch error")]
    BillFetchError,
}

const USER_AGENT_STR: &str =
    "Mozilla/5.0 (X11; Linux x86_64; rv:109.0) Gecko/20100101 Firefox/114.0";

#[derive(Default, Debug)]
pub struct Usage {
    pub mobile_data_used: Option<f64>,
    pub call_used: Option<isize>,
    pub sms_used: Option<isize>,
}

#[derive(Default, Debug)]
pub struct Bill {
    pub usage: isize,
}

impl UmobileClient {
    pub async fn new(id: impl AsRef<str>, password: impl AsRef<str>) -> Result<Self, ClientError> {
        let id = id.as_ref().to_string();
        let password = password.as_ref().to_string();

        let client = reqwest::Client::builder()
            .cookie_store(true)
            .build()
            .map_err(|_| ClientError::LoginError)?;

        let form = reqwest::multipart::Form::new()
            .text("username", id)
            .text("password", password)
            .text("loginMode", "idpw")
            .text("nextPage", "/")
            .text("autoLogin", "false");

        let res = client
            .post("https://www.uplusumobile.com/login-act")
            .header(USER_AGENT, USER_AGENT_STR)
            .multipart(form)
            .send()
            .await
            .map_err(|_| ClientError::LoginError)?;

        if res.status() != reqwest::StatusCode::OK {
            return Err(ClientError::LoginError);
        }

        Ok(Self { client })
    }

    pub async fn get_realtime_usage(&self) -> Result<Usage, ClientError> {
        let html = self
            .client
            .get("https://www.uplusumobile.com/my/usage/realTime")
            .header(USER_AGENT, USER_AGENT_STR)
            .send()
            .await
            .map_err(|_| ClientError::UsageFetchError)?
            .text()
            .await
            .map_err(|_| ClientError::UsageFetchError)?;

        let doc = Html::parse_document(&html);

        let item_box_sel = Selector::parse("section.box-usage-wrap").unwrap();
        let title_sel = Selector::parse("strong.usage-title").unwrap();
        let usage_box_sel = Selector::parse("div.usage-amount").unwrap();

        let mut usage = Usage::default();

        for item_box in doc.select(&item_box_sel) {
            let title = item_box.select(&title_sel).next();
            let title: String = if title.is_some() {
                title.unwrap()
            } else {
                continue;
            }
            .text()
            .collect();

            let usage_box = item_box.select(&usage_box_sel).next();
            let usage_text: String = if let Some(b) = usage_box {
                b.text().collect()
            } else {
                continue;
            };
            let usage_text = usage_text.replace("남음", "").replace("사용", "");

            if title.contains("데이터") {
                let usage_text = usage_text.replace("GB", "");
                let (_, used) = match usage_text.split_once("/") {
                    Some(s) => s,
                    None => continue,
                };

                let used = used
                    .trim()
                    .parse::<f64>()
                    .map_err(|_| ClientError::UsageFetchError)?;

                usage.mobile_data_used = Some(used);
            } else if title.contains("음성통화") {
                let usage_text = usage_text.replace("분", "");
                let (_, used) = match usage_text.split_once("/") {
                    Some(s) => s,
                    None => continue,
                };

                let used = used
                    .trim()
                    .parse::<isize>()
                    .map_err(|_| ClientError::UsageFetchError)?;

                usage.call_used = Some(used);
            } else if title.contains("메시지") {
                let usage_text = usage_text.replace("건", "");
                let (_, used) = match usage_text.split_once("/") {
                    Some(s) => s,
                    None => continue,
                };

                let used = used
                    .trim()
                    .parse::<isize>()
                    .map_err(|_| ClientError::UsageFetchError)?;

                usage.sms_used = Some(used);
            } else {
                continue;
            }
        }

        return Ok(usage);
    }

    pub async fn get_realtime_bill(&self) -> Result<Bill, ClientError> {
        let html = self
            .client
            .get("https://www.uplusumobile.com/my/usage/bill/detail-info")
            .header(USER_AGENT, USER_AGENT_STR)
            .send()
            .await
            .map_err(|_| ClientError::UsageFetchError)?
            .text()
            .await
            .map_err(|_| ClientError::UsageFetchError)?;

        let doc = Html::parse_document(&html);

        let item_box_sel = Selector::parse("div.info-area").unwrap();
        let detail_sel = Selector::parse("div.detail").unwrap();

        let item_box = doc.select(&item_box_sel).next();
        let item_box = if let Some(b) = item_box {
            b
        } else {
            return Err(ClientError::BillFetchError);
        };

        let detail = item_box.select(&detail_sel).next();
        let detail = if let Some(d) = detail {
            d
        } else {
            return Err(ClientError::BillFetchError);
        };

        let usage: String = detail.text().collect();
        let usage = usage.replace("원", "").replace(",", "");

        let usage = usage
            .trim()
            .parse::<isize>()
            .map_err(|_| ClientError::BillFetchError)?;

        Ok(Bill { usage })
    }
}
