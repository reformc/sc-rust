use serde::{Deserialize, Serialize};
use std::error::Error;

use crate::error;

//将gps数据转为struct
pub fn parse_gps(line: &str) -> Result<GpsInfo, Box<dyn Error>> {
    let a = line.split(", ").collect::<Vec<&str>>()[1].clone();
    let a = a.split(" ").collect::<Vec<&str>>();
    let id = a[0].parse::<u16>()?;
    let gps = a[2].clone();
    gps_type(id, gps)
}

//根据gps串口模式的数据格式转为struct
fn gps_type(id: u16, info: &str) -> Result<GpsInfo, Box<dyn Error>> {
    let a = info.split("\\c").collect::<Vec<&str>>();
    log::debug!("{},,{:?}", info, a);
    if a.len()<3{
        return Err(error::CustomizeError::new(-1, info))
    }
    if a[2] != "A" {
        return Err(error::CustomizeError::new(-1, "gps status is not A"));
    }
    match a[0] as &str {
        "$GPRMC" | "$GNRMC" => {
            if a.len()<9{
                return Err(error::CustomizeError::new(-1, info))
            }
            Ok(GpsInfo {
                id,
                lon: parse_gps_wgs84(&a[5])?,
                lat: parse_gps_wgs84(&a[3])?,
                vel: a[7].parse::<f32>().unwrap_or(0.0) * 1.852,
                ang: a[8].parse::<f32>().unwrap_or(0.0),
                //time:parse_time(&a[1], &a[10])
            })
        }
        "$GPGGA" => {
            if a.len()<5{
                return Err(error::CustomizeError::new(-1, info))
            }            
            Ok(GpsInfo {
                id,
                lon: parse_gps_wgs84(&a[4])?,
                lat: parse_gps_wgs84(&a[2])?,
                vel: 0.0,
                ang: 0.0,
            })
        },
        "$BDGSV" | "$GNGGA" | "$GPGSV" => Err(error::CustomizeError::new(
            -1,
            "$BDGSV,$GNGGA,$GPGSV can not parse error",
        )),
        _ => Err(error::CustomizeError::new(-1, info)),
    }
}
/*
fn parse_time(t:&str,d:&str)->String{
    let hh = t[0..2].to_string();
    let mm=t[2..4].to_string();
    let ss = t[4..].to_string();
    let dd= d[0..2].to_string();
    let mm_=d[2..4].to_string();
    let yy = d[4..].to_string();
    format!("20{}-{}-{}T{}:{}:{}Z",yy,mm_,dd,hh,mm,ss)
}
*/

//经纬度转wgs84格式
fn parse_gps_wgs84(data: &str) -> Result<f32, Box<dyn Error>> {
    match data.split(".").collect::<Vec<&str>>()[0].len() {
        5 => Ok(data[0..3].parse::<f32>()? + data[3..].parse::<f32>()? / 60.0),
        4 => Ok(data[0..2].parse::<f32>()? + data[2..].parse::<f32>()? / 60.0),
        _ => Err(error::CustomizeError::new(-1, "gps parse error")),
    }
}

//设备上下线时间转struct
pub fn parse_state(data: &str) -> Result<State, Box<dyn Error>> {
    let a = data.split(", ").collect::<Vec<&str>>();
    if a.len() < 2 {
        return Err(error::CustomizeError::new(-1, "state parse fail"));
    }
    let a = a[1].split(" ").collect::<Vec<&str>>();
    if a.len() < 2 {
        return Err(error::CustomizeError::new(-1, "state parse fail"));
    }
    Ok(State {
        id: a[0].parse::<u16>()?,
        online: a[1] == "0",
    })
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GpsInfo {
    pub id: u16,
    pub lon: f32,
    pub lat: f32,
    pub vel: f32,
    pub ang: f32,
    //pub time:String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct State {
    pub id: u16,
    pub online: bool,
}
