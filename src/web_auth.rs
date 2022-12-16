use std::collections::HashMap;
use async_trait::async_trait;
use axum::{extract::FromRequestParts, http::{StatusCode, request::Parts}};

/*
websocket鉴权方式：
header携带AUTHORIZATION为用户登录名,cookie携带sessionid
后台收到后去tbl_sys_session表查询session是否正确，session有效期一小时。
*/
pub struct RequireAuth;
#[async_trait]
impl<S> FromRequestParts<S> for RequireAuth
where
    S: Send+Sync,
{
    type Rejection = StatusCode;//Redirect; //

    async fn from_request_parts(req: &mut Parts,_: &S) -> Result<Self, Self::Rejection> {//state
        let auth_header = req
        .headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok()).unwrap_or("");
        let cookie_str = req
        .headers
        .get(axum::http::header::COOKIE)
        .and_then(|value| value.to_str().ok()).unwrap_or("");
        let cookie = Cookie::new(cookie_str);
        let session_id = cookie.get("JSESSIONID").unwrap_or("null");
        log::info!("session_id: {}",session_id);
        if check_session_id(auth_header, session_id).await{
                return Ok(Self);
        }
        log::warn!("auth error");
        //Ok(Self)
        Err(StatusCode::UNAUTHORIZED)
        //Err(Redirect::to(Uri::from_static("/login")))
    }
}

struct Cookie<'a>{
    list:HashMap<&'a str,&'a str>
}

impl<'a> Cookie<'a>{
    fn new(cookie_str:&str)->Cookie{
        let mut list = HashMap::new();
        let cookies:Vec<&str> = cookie_str.split(";").collect();
        for cookie in cookies{
            let cookie_pair:Vec<&str> = cookie.split("=").collect();
            list.insert(cookie_pair[0].trim(), cookie_pair[1].trim());
        }
        Cookie { list }
    }

    fn get(self,key:&str)->Option<&'a str>{
        self.list.get(key).copied()
    }
}

//此处仅为示范，并无实际意义，可自行修改为有意义的鉴权代码。
async fn check_session_id(user:&str,session_id:&str)->bool{
    session_id=="test" || user==session_id
}