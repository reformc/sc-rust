use core::fmt;

#[derive(Debug)]
pub struct CustomizeError{
    kind:i32,
    msg:String
}

impl CustomizeError {
    pub fn new(kind:i32,message:&str)->Box<dyn std::error::Error>{
        Box::new(CustomizeError {
            kind,
            msg: message.to_string()
        })
    }
}

impl fmt::Display for CustomizeError{
    fn fmt(&self,f:&mut fmt::Formatter)->fmt::Result{
        write!(f,"code:{},message:{}",self.kind,&self.msg)
    }
}

impl std::error::Error for CustomizeError {}

