use prometheus::{self, register_int_gauge_vec, Encoder, TextEncoder};
use reqwest;
use scraper::{
    self,
    error::{self, SelectorErrorKind},
    Html,
    Selector};
use thiserror::Error;

const ARSO_URL: &str = "https://meteo.arso.gov.si/uploads/probase/www/observ/surface/text/sl/observationAms_si_latest.html";

#[derive(Error, Debug)]
pub enum ArsoError {
    #[error("Connection error")]
    Connection,
    #[error("Parsing error")]
    ParseError(String),
}

impl std::convert::From<SelectorErrorKind<'_>> for ArsoError {
    fn from(err: SelectorErrorKind) -> Self {
        ArsoError::ParseError(err.to_string())
    }
}

fn arso_get_document() -> Result<String, ArsoError> {
    /*
        let res = reqwest::blocking::get(ARSO_URL)?;

        println!("Status: {}", res.status());
        println!("Headers:\n{:#?}", res.headers());
        println!("Body:\n{}", res.text()? );
    */

    let body = include_str!("../inp.html");
    Ok(body.to_string())
}

pub fn arso_get_metrics() -> Result<String, ArsoError> {
    let g_temp = register_int_gauge_vec!("arso_temperature",
        "Temperature",
        &["city"]).unwrap();
    let g_rh = register_int_gauge_vec!("arso_relative_humidity",
        "Relative humidity",
        &["city"]).unwrap();

    let body = arso_get_document()?;

    let doc = Html::parse_document(&body);
    let sel_dt = Selector::parse("th.meteoSI-header").unwrap();
    let sel_row = Selector::parse("table.meteoSI-table > tbody > tr").unwrap();
    let sel_city = Selector::parse("td.meteoSI-th").unwrap();
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

    let node = doc.select(&sel_dt).next().unwrap();
    dbg!(node.inner_html());

    let nodes = doc.select(&sel_row).collect::<Vec<_>>();
    for node in nodes {
        let city = node.select(&sel_city).next().unwrap().inner_html();
        //let icon = node.select(&sel_icon).next().unwrap();
        let temp = node
            .select(&sel_temp)
            .next()
            .unwrap()
            .inner_html()
            .parse::<i64>()
            .unwrap_or(-50);
        let rh = node
            .select(&sel_rh)
            .next()
            .unwrap()
            .inner_html()
            .parse::<i64>()
            .unwrap_or(-50);
        //let ddavg_icon = node.select(&sel_ddavg_icon).next().unwrap();
        //let ffavg = node.select(&sel_ffavg).next().unwrap();
        //let ffmax = node.select(&sel_ffmax).next().unwrap();
        //let msl = node.select(&sel_msl).next().unwrap();
        //let rr = node.select(&sel_rr).next().unwrap();
        //let gsunradavg = node.select(&sel_g_sun_radavg).next().unwrap();
        //let snow = node.select(&sel_snow).next().unwrap();

        g_temp.with_label_values(&[&city]).set(temp);
        g_rh.with_label_values(&[&city]).set(rh);
    }

    let encoder = TextEncoder::new();
    let mut buffer = vec![];
    encoder
        .encode(&prometheus::gather(), &mut buffer)
        .expect("Failed to encode metrics");

    let response = String::from_utf8(buffer.clone()).expect("Failed to convert bytes to string");
    buffer.clear();

    Ok(response)
}
