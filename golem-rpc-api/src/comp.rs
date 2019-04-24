use super::{Map, Result};
use serde_derive::*;
use wamp_derive::*;

#[wamp_interface]
pub trait GolemComp {}
