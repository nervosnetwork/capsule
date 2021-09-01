use jsonrpc_core::error::Error as JsonRpcError;
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};

use ckb_testtool::ckb_error::AnyError;
use ckb_testtool::ckb_jsonrpc_types::{
    BlockNumber, BlockView, CellWithStatus, OutPoint, Transaction, TransactionWithStatus,
};
use ckb_testtool::ckb_types::{
    core::BlockNumber as CoreBlockNumber, packed::Byte32, prelude::*, H256,
};

lazy_static::lazy_static! {
    pub static ref HTTP_CLIENT: reqwest::blocking::Client = reqwest::blocking::Client::builder()
        .timeout(::std::time::Duration::from_secs(30))
        .build()
        .expect("reqwest Client build");
}

#[derive(Debug)]
pub struct IdGenerator {
    state: AtomicU64,
}

impl Default for IdGenerator {
    fn default() -> Self {
        IdGenerator {
            state: AtomicU64::new(1),
        }
    }
}

impl IdGenerator {
    pub fn new() -> IdGenerator {
        IdGenerator::default()
    }

    pub fn next(&self) -> u64 {
        self.state.fetch_add(1, Ordering::SeqCst)
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Error {
    pub(in crate::wallet) inner: JsonRpcError,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            serde_json::to_string(&self.inner).expect("JsonRpcError to_string")
        )
    }
}

impl ::std::error::Error for Error {}

macro_rules! jsonrpc {
    (
        $(#[$struct_attr:meta])*
        pub struct $struct_name:ident {$(
            $(#[$attr:meta])*
            pub fn $method:ident(&$selff:ident $(, $arg_name:ident: $arg_ty:ty)*)
                -> $return_ty:ty;
        )*}
    ) => (
        $(#[$struct_attr])*
        pub struct $struct_name {
            pub client: &'static reqwest::blocking::Client,
            pub url: reqwest::Url,
            pub id_generator: $crate::wallet::rpc::IdGenerator,
        }

        impl $struct_name {
            pub fn new(uri: &str) -> Self {
                let url = reqwest::Url::parse(uri).expect("ckb uri, e.g. \"http://127.0.0.1:8114\"");
                let id_generator = $crate::wallet::rpc::IdGenerator::new();
                $struct_name { url, id_generator, client: &$crate::wallet::rpc::HTTP_CLIENT, }
            }

            $(
                $(#[$attr])*
                pub fn $method(&$selff $(, $arg_name: $arg_ty)*) -> Result<$return_ty, ckb_testtool::ckb_error::AnyError> {
                    let method = String::from(stringify!($method));
                    let params = serialize_parameters!($($arg_name,)*);
                    let id = $selff.id_generator.next();

                    let mut req_json = serde_json::Map::new();
                    req_json.insert("id".to_owned(), serde_json::json!(id));
                    req_json.insert("jsonrpc".to_owned(), serde_json::json!("2.0"));
                    req_json.insert("method".to_owned(), serde_json::json!(method));
                    req_json.insert("params".to_owned(), params);

                    let resp = $selff.client.post($selff.url.clone()).json(&req_json).send()?;
                    let output = resp.json::<jsonrpc_core::response::Output>()?;
                    match output {
                        jsonrpc_core::response::Output::Success(success) => {
                            serde_json::from_value(success.result).map_err(Into::into)
                        },
                        jsonrpc_core::response::Output::Failure(failure) => {
                            Err($crate::wallet::rpc::Error{ inner: failure.error }.into())
                        }
                    }
                }
            )*
        }
    )
}

macro_rules! serialize_parameters {
    () => ( serde_json::Value::Null );
    ($($arg_name:ident,)+) => ( serde_json::to_value(($($arg_name,)+))?)
}

pub struct RpcClient {
    inner: Inner,
}

impl RpcClient {
    pub fn new(uri: &str) -> Self {
        Self {
            inner: Inner::new(uri),
        }
    }

    // pub fn inner(&self) -> &Inner {
    //     &self.inner
    // }

    pub fn get_transaction(&self, hash: Byte32) -> Option<TransactionWithStatus> {
        self.inner
            .get_transaction(hash.unpack())
            .expect("rpc call get_transaction")
    }
    pub fn send_transaction(&self, tx: Transaction) -> Byte32 {
        self.send_transaction_result(tx)
            .expect("rpc call send_transaction")
            .pack()
    }
    pub fn send_transaction_result(&self, tx: Transaction) -> Result<H256, AnyError> {
        self.inner
            .send_transaction(tx, Some("passthrough".to_string()))
    }
    pub fn get_live_cell(
        &self,
        out_point: OutPoint,
        with_data: bool,
    ) -> Result<CellWithStatus, AnyError> {
        self.inner.get_live_cell(out_point, with_data)
    }
    pub fn get_block_by_number(&self, number: CoreBlockNumber) -> Option<BlockView> {
        self.inner
            .get_block_by_number(number.into())
            .expect("rpc call get_block_by_number")
    }
}

jsonrpc!(pub struct Inner {
    pub fn get_transaction(&self, _hash: H256) -> Option<TransactionWithStatus>;
    pub fn send_transaction(&self, tx: Transaction, outputs_validator: Option<String>) -> H256;
    pub fn get_live_cell(&self, _out_point: OutPoint, _with_data: bool) -> CellWithStatus;
    pub fn get_block_by_number(&self, _number: BlockNumber) -> Option<BlockView>;
});
