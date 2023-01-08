use prometheus::{self, register_int_gauge_vec, Encoder, TextEncoder, IntGaugeVec};
use reqwest;
use scraper::{
    self,
    error::SelectorErrorKind,
    Html,
    Selector,
    ElementRef,
};
use thiserror::Error;
use lazy_static::lazy_static;

const ARSO_URL: &str = "https://meteo.arso.gov.si/uploads/probase/www/observ/surface/text/sl/observationAms_si_latest.html";

#[derive(Error, Debug)]
pub enum ArsoError {
    #[error("Connection error: {0}")]
    ConnectionError(String),
    #[error("Parsing error: {0}")]
    ParseError(String),
}

impl std::convert::From<SelectorErrorKind<'_>> for ArsoError {
    fn from(err: SelectorErrorKind) -> Self {
        ArsoError::ParseError(err.to_string())
    }
}

impl std::convert::From<reqwest::Error> for ArsoError {
    fn from(err: reqwest::Error) -> Self {
        ArsoError::ConnectionError(err.to_string())
    }
}

#[derive(Debug)]
struct City {
    name: String,
    temperature: i32,
    relative_humidity: u32,
}

async fn arso_get_document() -> Result<String, ArsoError> {

    let body = reqwest::get(ARSO_URL)
        .await?
        .text()
        .await?;
    
    Ok(body)
}

fn parse_datetime(input: &Html) -> Result<String, ArsoError> {
    let sel_dt = Selector::parse("th.meteoSI-header").unwrap();

    let datetime = input
        .select(&sel_dt)
        .next()
        .ok_or(ArsoError::ParseError("Datetime not found".to_string()))?
        .inner_html();

    Ok(datetime)
}

fn parse_city(input: &ElementRef) -> Result<City, ArsoError> {

    let sel_name = Selector::parse("td.meteoSI-th").unwrap();
    //let sel_icon = Selector::parse("td.nn_icon_wwsyn_icon").unwrap();
    let sel_temp = Selector::parse("td.t").unwrap();
    let sel_rh = Selector::parse("td.rh").unwrap();
    //let sel_ddavg_icon = Selector::parse("td.ddavg_icon").unwrap();
    //let sel_ffavg = Selector::parse("td.ffavg_val").unwrap();
    //let sel_ffmax = Selector::parse("td.ffmax_val").unwrap();
    //let sel_msl = Selector::parse("td.msl").unwrap();
    //let sel_rr = Selector::parse("td.rr_val").unwrap();
    //let sel_g_sun_radavg = Selector::parse("td.gSunRadavg").unwrap();
    //let sel_snow = Selector::parse("td.snow").unwrap();

    let name = input.select(&sel_name).next().unwrap().inner_html();
    //let icon = node.select(&sel_icon).next().unwrap();
    let temperature = input
        .select(&sel_temp)
        .next()
        .ok_or(ArsoError::ParseError("Temperature field not found".to_string()))?
        .inner_html()
        .parse::<i32>()
        .unwrap_or(-50);
    let relative_humidity = input 
        .select(&sel_rh)
        .next()
        .unwrap()
        .inner_html()
        .parse::<u32>()
        .unwrap_or(0);
    //let ddavg_icon = node.select(&sel_ddavg_icon).next().unwrap();
    //let ffavg = node.select(&sel_ffavg).next().unwrap();
    //let ffmax = node.select(&sel_ffmax).next().unwrap();
    //let msl = node.select(&sel_msl).next().unwrap();
    //let rr = node.select(&sel_rr).next().unwrap();
    //let gsunradavg = node.select(&sel_g_sun_radavg).next().unwrap();
    //let snow = node.select(&sel_snow).next().unwrap();

//    g_temp.with_label_values(&[&city]).set(temp);
//    g_rh.with_label_values(&[&city]).set(rh);
    Ok(
        City {
            name,
            temperature,
            relative_humidity
        }
    )
}

lazy_static! {
    pub static ref GAUGE_TEMPERATURE: IntGaugeVec = register_int_gauge_vec!(
        "arso_temperature",
        "Temperature",
        &["city"]
    ).unwrap();
    static ref GAUGE_RH: IntGaugeVec = register_int_gauge_vec!(
        "arso_relative_humidity",
        "Relative humidity",
        &["city"]
    ).unwrap();
}

pub async fn arso_retrieve() -> Result<(), ArsoError> {

    println!("Retrieving data from ARSO");

    let body = arso_get_document().await?;
    let doc = Html::parse_document(&body);
    let sel_row = Selector::parse("table.meteoSI-table > tbody > tr").unwrap();

    let dt = parse_datetime(&doc);
    println!("Current timestamp: {}", dt?);

    let nodes = doc.select(&sel_row).collect::<Vec<_>>();
    for node in nodes {
        let city = parse_city(&node)?;

        GAUGE_TEMPERATURE.with_label_values(&[&city.name]).set(city.temperature as i64);
        GAUGE_RH.with_label_values(&[&city.name]).set(city.relative_humidity as i64);
    }
    
    Ok(())
}
pub async fn arso_get_metrics() -> Result<String, ArsoError> {

    let encoder = TextEncoder::new();
    let mut buffer = vec![];
    encoder
        .encode(&prometheus::gather(), &mut buffer)
        .expect("Failed to encode metrics");

    let response = String::from_utf8(buffer.clone()).expect("Failed to convert bytes to string");
    buffer.clear();

    Ok(response)
}
