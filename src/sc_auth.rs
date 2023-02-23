use encoding::all::GBK;
use encoding::{DecoderTrap, Encoding};
use serde::{Deserialize, Serialize};
use std::{error::Error, sync::Arc};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    time::{timeout, Duration},
};

const TIMEOUT_SECS: u64 = 3;

#[derive(Serialize, Deserialize, Debug)]
pub struct DeviceGather {
    pub groups: Vec<Group>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Group {
    pub id: usize,
    pub name: String,
    pub des: String,
    pub devices: Vec<Device>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Device {
    pub id: u16,
    pub name: String,
    pub state: u8,
}

//获取此用户权限下所有的设备分组。
pub async fn device_info(
    user: Arc<String>,
    addr: Arc<String>,
) -> Result<DeviceGather, Box<dyn Error>> {
    let user = (*user).clone();
    let addr = (*addr).clone();
    let mut stream = TcpStream::connect(&addr).await?;
    stream.writable().await?;
    stream
        .write_all(format!("GETCG {}, {}\r\n", 23177, &user).as_bytes())
        .await?;
    let mut buf = [0; 4];
    stream.readable().await?;
    let n = stream.read(&mut buf).await?;
    let sum = little_num(&buf[..n]);
    let mut groups = vec![];
    for _ in 0..sum {
        let mut buf = [0; 164];
        stream.readable().await?;
        let _ = timeout(
            Duration::from_secs(TIMEOUT_SECS),
            stream.read_exact(&mut buf),
        )
        .await??;
        let group_id = little_num(&buf[0..4]);
        let group_name = GBK
            .decode(&buf[8..58], DecoderTrap::Strict)?
            .replace("\0", "");
        let group_des = GBK
            .decode(&buf[58..], DecoderTrap::Strict)?
            .replace("\0", "");
        let devices = get_camer_info(group_id, &user, &addr).await?;
        groups.push(Group {
            id: group_id,
            name: group_name,
            des: group_des,
            devices,
        });
    }
    Ok(DeviceGather { groups })
}

//获取设备分组里，用户权限下的设备。
async fn get_camer_info(
    group_id: usize,
    user: &str,
    addr: &str,
) -> Result<Vec<Device>, Box<dyn Error>> {
    let mut stream = TcpStream::connect(addr).await?;
    stream.writable().await?;
    stream
        .write_all(format!("GETCAMERINCG {}, {} {}\r\n", 23177, group_id, user).as_bytes())
        .await?;
    let mut buf = [0; 4];
    stream.readable().await?;
    let n = stream.read(&mut buf).await?;
    let sum = little_num(&buf[..n]);
    let mut devices = vec![];
    for _ in 0..sum {
        let mut buf = [0; 188];
        stream.readable().await?;
        let n = timeout(
            Duration::from_secs(TIMEOUT_SECS),
            stream.read_exact(&mut buf),
        )
        .await??;
        let device_id = little_num(&buf[..2]);
        let device_name = GBK
            .decode(&buf[32..82], DecoderTrap::Strict)?
            .replace("\0", "");
        let device_stat = &buf[n - 4];
        devices.push(Device {
            id: device_id as u16,
            name: device_name,
            state: device_stat.clone(),
        });
    }
    Ok(devices)
}

fn little_num(data: &[u8]) -> usize {
    let mut res = 0;
    let mut i = 0;
    let bit: usize = 256;
    for c in data {
        res += *c as usize * (bit.pow(i));
        i += 1;
    }
    res
}
