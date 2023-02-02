use std::num::{ParseFloatError, ParseIntError};

use once_cell::sync::Lazy;
use prometheus::{self, register_gauge_vec, Encoder, GaugeVec, TextEncoder};
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

impl std::convert::From<ParseIntError> for ArsoError {
    fn from(err: ParseIntError) -> Self {
        ArsoError::ParseError(err.to_string())
    }
}

impl std::convert::From<ParseFloatError> for ArsoError {
    fn from(err: ParseFloatError) -> Self {
        ArsoError::ParseError(err.to_string())
    }
}

#[derive(Default, Debug)]
struct City {
    name: String,
    temperature: Option<f64>,
    relative_humidity: Option<f64>,
    wind_avg: Option<f64>,
    wind_max: Option<f64>,
    rainfall: Option<f64>,
    solar: Option<f64>,
    snowfall: Option<f64>,
}

impl City {
    fn new() -> Self {
        City {
            ..Default::default()
        }
    }

    fn set_temperature(city: &mut Self, value: Option<f64>) {
        city.temperature = value;
    }
    fn set_relative_humidity(city: &mut Self, value: Option<f64>) {
        city.relative_humidity = value;
    }
    fn set_wind_avg(city: &mut Self, value: Option<f64>) {
        city.wind_avg = value;
    }
    fn set_wind_max(city: &mut Self, value: Option<f64>) {
        city.wind_max = value;
    }
    fn set_rainfall(city: &mut Self, value: Option<f64>) {
        city.rainfall = value;
    }
    fn set_solar(city: &mut Self, value: Option<f64>) {
        city.solar = value;
    }
    fn set_snowfall(city: &mut Self, value: Option<f64>) {
        city.snowfall = value;
    }

    fn get_temperature(city: &Self) -> Option<f64> {
        city.temperature
    }
    fn get_relative_humidity(city: &Self) -> Option<f64> {
        city.relative_humidity
    }
    fn get_wind_avg(city: &Self) -> Option<f64> {
        city.wind_avg
    }
    fn get_wind_max(city: &Self) -> Option<f64> {
        city.wind_max
    }
    fn get_rainfall(city: &Self) -> Option<f64> {
        city.rainfall
    }
    fn get_solar(city: &Self) -> Option<f64> {
        city.solar
    }
    fn get_snowfall(city: &Self) -> Option<f64> {
        city.snowfall
    }
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

//#[derive()]
struct ArsoField {
    selector_tag: String,
    field_name: String,
    set_fn: fn(&mut City, Option<f64>),
    get_fn: fn(&City) -> Option<f64>,
    metric: GaugeVec,
}

static ARSO_FIELDS: Lazy<Vec<ArsoField>> = Lazy::new(|| {
    let fields: Vec<ArsoField> = vec![
        ArsoField {
            selector_tag: "td.t".to_string(),
            field_name: "Temperature".to_string(),
            set_fn: City::set_temperature,
            get_fn: City::get_temperature,
            metric: register_gauge_vec!("arso_temperature", "Temperature", &["city"]).unwrap(),
        },
        ArsoField {
            selector_tag: "td.rh".to_string(),
            field_name: "Relative humidity".to_string(),
            set_fn: City::set_relative_humidity,
            get_fn: City::get_relative_humidity,
            metric: register_gauge_vec!("arso_relative_humidity", "Relative humidity", &["city"])
                .unwrap(),
        },
        ArsoField {
            selector_tag: "td.ffavg_val".to_string(),
            field_name: "Average wind speed".to_string(),
            set_fn: City::set_wind_avg,
            get_fn: City::get_wind_avg,
            metric: register_gauge_vec!("arso_wind_avg", "Average wind speed", &["city"]).unwrap(),
        },
        ArsoField {
            selector_tag: "td.ffmax_val".to_string(),
            field_name: "Max wind speed".to_string(),
            set_fn: City::set_wind_max,
            get_fn: City::get_wind_max,
            metric: register_gauge_vec!("arso_wind_max", "Maximum wind speed", &["city"]).unwrap(),
        },
        ArsoField {
            selector_tag: "td.rr_val".to_string(),
            field_name: "Rainfall".to_string(),
            set_fn: City::set_rainfall,
            get_fn: City::get_rainfall,
            metric: register_gauge_vec!("arso_rainfall", "Rainfall", &["city"]).unwrap(),
        },
        ArsoField {
            selector_tag: "td.gSunRadavg".to_string(),
            field_name: "Solar radiation".to_string(),
            set_fn: City::set_solar,
            get_fn: City::get_solar,
            metric: register_gauge_vec!("arso_solar_radiation", "Solar radiation", &["city"])
                .unwrap(),
        },
        ArsoField {
            selector_tag: "td.snow".to_string(),
            field_name: "Snow blanket depth".to_string(),
            set_fn: City::set_snowfall,
            get_fn: City::get_snowfall,
            metric: register_gauge_vec!("arso_snowfall", "Snow blanket depth", &["city"]).unwrap(),
        },
    ];
    fields
});

fn parse_city(input: &ElementRef) -> Result<City, ArsoError> {
    let mut city = City::new();

    let sel_name = Selector::parse("td.meteoSI-th").unwrap();
    let name = input.select(&sel_name).next().unwrap().inner_html();
    city.name = name;

    for field in ARSO_FIELDS.iter() {
        let selector = Selector::parse(&field.selector_tag).unwrap();
        let val = input
            .select(&selector)
            .next()
            .ok_or(ArsoError::ParseError(format!(
                "Field '{}' not found",
                field.field_name
            )))?
            .inner_html()
            .parse::<f64>()
            .ok();

        (field.set_fn)(&mut city, val);
    }

    Ok(city)
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
            for field in ARSO_FIELDS.iter() {
                let val = (field.get_fn)(&city);

                if let Some(val) = val {
                    field.metric.with_label_values(&[&city.name]).set(val);
                } else {
                    let _ = field.metric.remove_label_values(&[&city.name]);
                }
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
