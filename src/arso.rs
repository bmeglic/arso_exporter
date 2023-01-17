use std::num::{ParseFloatError, ParseIntError};

use lazy_static::lazy_static;
use prometheus::{
    self, register_gauge_vec, register_int_gauge_vec, Encoder, GaugeVec, IntGaugeVec, TextEncoder,
};
use reqwest;
use scraper::{self, error::SelectorErrorKind, ElementRef, Html, Selector};
use thiserror::Error;

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
    temperature: Result<i32, ParseIntError>,
    relative_humidity: Result<u32, ParseIntError>,
    wind_avg: Result<u32, ParseIntError>,
    rainfall: Result<f64, ParseFloatError>,
    solar: Result<u32, ParseIntError>,
    snowfall: Result<u32, ParseIntError>,
}

async fn arso_get_document() -> Result<String, ArsoError> {
    let body = reqwest::get(ARSO_URL).await?.text().await?;
    //let body = include_str!("../inp.html").to_string();

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
    let sel_ffavg = Selector::parse("td.ffavg_val").unwrap();
    //let sel_ffmax = Selector::parse("td.ffmax_val").unwrap();
    //let sel_msl = Selector::parse("td.msl").unwrap();
    let sel_rr = Selector::parse("td.rr_val").unwrap();
    let sel_g_sun_radavg = Selector::parse("td.gSunRadavg").unwrap();
    let sel_snow = Selector::parse("td.snow").unwrap();

    let name = input.select(&sel_name).next().unwrap().inner_html();
    //let icon = node.select(&sel_icon).next().unwrap();
    let temperature = input
        .select(&sel_temp)
        .next()
        .ok_or(ArsoError::ParseError(
            "Temperature field not found".to_string(),
        ))?
        .inner_html()
        .parse::<i32>();
    let relative_humidity = input
        .select(&sel_rh)
        .next()
        .ok_or(ArsoError::ParseError(
            "Relative humidity field not found".to_string(),
        ))?
        .inner_html()
        .parse::<u32>();
    //let ddavg_icon = node.select(&sel_ddavg_icon).next().unwrap();
    let wind_avg = input
        .select(&sel_ffavg)
        .next()
        .ok_or(ArsoError::ParseError(
            "Average wind field not found".to_string(),
        ))?
        .inner_html()
        .parse::<u32>();
    //let ffmax = node.select(&sel_ffmax).next().unwrap();
    //let msl = node.select(&sel_msl).next().unwrap();
    let rainfall = input
        .select(&sel_rr)
        .next()
        .ok_or(ArsoError::ParseError(
            "Rainfall field not found".to_string(),
        ))?
        .inner_html()
        .parse::<f64>();

    let solar = input
        .select(&sel_g_sun_radavg)
        .next()
        .ok_or(ArsoError::ParseError(
            "Solar radiation field not found".to_string(),
        ))?
        .inner_html()
        .parse::<u32>();
    let snowfall = input
        .select(&sel_snow)
        .next()
        .ok_or(ArsoError::ParseError("Snow field not found".to_string()))?
        .inner_html()
        .parse::<u32>();

    Ok(City {
        name,
        temperature,
        relative_humidity,
        rainfall,
        wind_avg,
        solar,
        snowfall,
    })
}

lazy_static! {
    static ref GAUGE_TEMPERATURE: IntGaugeVec =
        register_int_gauge_vec!("arso_temperature", "Temperature", &["city"]).unwrap();
    static ref GAUGE_RH: IntGaugeVec =
        register_int_gauge_vec!("arso_relative_humidity", "Relative humidity", &["city"]).unwrap();
    static ref GAUGE_RAINFALL: GaugeVec =
        register_gauge_vec!("arso_rainfall", "Rainfall", &["city"]).unwrap();
    static ref GAUGE_WIND_AVG: IntGaugeVec =
        register_int_gauge_vec!("arso_wind_average", "Average wind speed", &["city"]).unwrap();
    static ref GAUGE_SOLAR: IntGaugeVec =
        register_int_gauge_vec!("arso_solar_radiation", "Solar radiation", &["city"]).unwrap();
    static ref GAUGE_SNOWFALL: IntGaugeVec =
        register_int_gauge_vec!("arso_snowfall", "Snow fall", &["city"]).unwrap();
}

pub async fn arso_retrieve(cities: &Vec<String>) -> Result<(), ArsoError> {
    println!("Retrieving data from ARSO");

    let body = arso_get_document().await?;
    let doc = Html::parse_document(&body);
    let sel_row = Selector::parse("table.meteoSI-table > tbody > tr").unwrap();

    let dt = parse_datetime(&doc);
    println!("Current timestamp: {}", dt?);

    let nodes = doc.select(&sel_row).collect::<Vec<_>>();
    for node in nodes {
        let city = parse_city(&node)?;

        if cities.contains(&city.name) {
            if let Ok(temperature) = city.temperature {
                GAUGE_TEMPERATURE
                    .with_label_values(&[&city.name])
                    .set(temperature as i64);
            } else {
                let _ = GAUGE_TEMPERATURE.remove_label_values(&[&city.name]);
            }

            if let Ok(relative_humidity) = city.relative_humidity {
                GAUGE_RH
                    .with_label_values(&[&city.name])
                    .set(relative_humidity as i64);
            } else {
                let _ = GAUGE_RH.remove_label_values(&[&city.name]);
            }

            if let Ok(rainfall) = city.rainfall {
                GAUGE_RAINFALL
                    .with_label_values(&[&city.name])
                    .set(rainfall);
            } else {
                let _ = GAUGE_RAINFALL.remove_label_values(&[&city.name]);
            }

            if let Ok(wind_avg) = city.wind_avg {
                GAUGE_WIND_AVG
                    .with_label_values(&[&city.name])
                    .set(wind_avg as i64);
            } else {
                let _ = GAUGE_WIND_AVG.remove_label_values(&[&city.name]);
            }

            if let Ok(solar) = city.solar {
                GAUGE_SOLAR
                    .with_label_values(&[&city.name])
                    .set(solar as i64);
            } else {
                let _ = GAUGE_SOLAR.remove_label_values(&[&city.name]);
            }

            if let Ok(snowfall) = city.snowfall {
                GAUGE_SNOWFALL
                    .with_label_values(&[&city.name])
                    .set(snowfall as i64);
            } else {
                let _ = GAUGE_SNOWFALL.remove_label_values(&[&city.name]);
            }
        }
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
