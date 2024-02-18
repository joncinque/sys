#[cfg(feature = "binance")]
use crate::binance_exchange;
#[cfg(feature = "coinbase")]
use crate::coinbase_exchange;
#[cfg(feature = "ftx")]
use crate::ftx_exchange;
use {
    crate::{kraken_exchange, token::MaybeToken},
    async_trait::async_trait,
    chrono::NaiveDate,
    serde::{Deserialize, Serialize},
    solana_sdk::pubkey::Pubkey,
    std::{collections::HashMap, str::FromStr},
    thiserror::Error,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Exchange {
    #[cfg(feature = "binance")]
    Binance,
    #[cfg(feature = "binance")]
    BinanceUs,
    #[cfg(feature = "coinbase")]
    Coinbase,
    #[cfg(feature = "ftx")]
    Ftx,
    #[cfg(feature = "ftx")]
    FtxUs,
    Kraken,
}

impl std::fmt::Display for Exchange {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

pub const USD_COINS: &[&str] = &["USD", "USDC", "USDT", "BUSD", "ZUSD"];

impl FromStr for Exchange {
    type Err = ParseExchangeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            #[cfg(feature = "binance")]
            "Binance" | "binance" => Ok(Exchange::Binance),
            #[cfg(feature = "binance")]
            "BinanceUs" | "binanceus" => Ok(Exchange::BinanceUs),
            #[cfg(feature = "coinbase")]
            "Coinbase" | "coinbase" => Ok(Exchange::Coinbase),
            #[cfg(feature = "ftx")]
            "Ftx" | "ftx" => Ok(Exchange::Ftx),
            #[cfg(feature = "ftx")]
            "FtxUs" | "ftxus" => Ok(Exchange::FtxUs),
            "Kraken" | "kraken" => Ok(Exchange::Kraken),
            _ => Err(ParseExchangeError::InvalidExchange),
        }
    }
}

#[derive(Error, Debug)]
pub enum ParseExchangeError {
    #[error("invalid exchange")]
    InvalidExchange,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExchangeCredentials {
    pub api_key: String,
    pub secret: String,
    pub subaccount: Option<String>,
}

#[derive(Debug, Default, Clone)]
pub struct ExchangeBalance {
    pub available: f64,
    pub total: f64,
}

#[derive(Debug)]
pub struct DepositInfo {
    pub tx_id: String,
    pub amount: f64, // TODO: rename to `ui_amount`
}

#[derive(Debug)]
pub struct WithdrawalInfo {
    pub address: Pubkey,
    pub token: MaybeToken,
    pub amount: f64, // TODO: rename to `ui_amount`
    pub tag: String,

    pub completed: bool, // when `completed`, a `tx_id` of `None` indicates a cancelled withdrawal
    pub tx_id: Option<String>,
}

#[derive(Debug)]
pub struct BidAsk {
    pub bid_price: f64,
    pub ask_price: f64,
}

pub type OrderId = String;

#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
pub enum OrderSide {
    Buy,
    Sell,
}

impl std::fmt::Display for OrderSide {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

#[derive(Debug)]
pub struct OrderStatus {
    pub open: bool,
    pub side: OrderSide,
    pub price: f64,
    pub amount: f64,
    pub filled_amount: f64,
    pub last_update: NaiveDate,
    pub fee: Option<(f64, String)>,
}

#[derive(PartialEq, Eq)]
pub enum MarketInfoFormat {
    All,
    Ask,
    Weighted24hAveragePrice,
    Hourly,
}

pub struct LendingInfo {
    pub lendable: f64,
    pub offered: f64,
    pub locked: f64,
    pub estimate_rate: f64, // estimated lending rate for the next spot margin cycle
    pub previous_rate: f64, // lending rate in the previous spot margin cycle
}

pub enum LendingHistory {
    Range {
        start_date: NaiveDate,
        end_date: NaiveDate,
    },
    Previous {
        days: usize,
    },
}

#[async_trait]
pub trait ExchangeClient {
    async fn deposit_address(
        &self,
        token: MaybeToken,
    ) -> Result<Pubkey, Box<dyn std::error::Error>>;
    async fn recent_deposits(&self)
        -> Result<Option<Vec<DepositInfo>>, Box<dyn std::error::Error>>;
    async fn recent_withdrawals(&self) -> Result<Vec<WithdrawalInfo>, Box<dyn std::error::Error>>;
    async fn request_withdraw(
        &self,
        address: Pubkey,
        token: MaybeToken,
        amount: f64,
        withdrawal_password: Option<String>,
        withdrawal_code: Option<String>,
    ) -> Result<(/* withdraw_id: */ String, /*withdraw_fee: */ f64), Box<dyn std::error::Error>>;
    async fn balances(
        &self,
    ) -> Result<HashMap<String, ExchangeBalance>, Box<dyn std::error::Error>>;
    async fn print_market_info(
        &self,
        pair: &str,
        format: MarketInfoFormat,
    ) -> Result<(), Box<dyn std::error::Error>>;
    async fn bid_ask(&self, pair: &str) -> Result<BidAsk, Box<dyn std::error::Error>>;
    async fn place_order(
        &self,
        pair: &str,
        side: OrderSide,
        price: f64,
        amount: f64,
    ) -> Result<OrderId, Box<dyn std::error::Error>>;
    #[allow(clippy::ptr_arg)]
    async fn cancel_order(
        &self,
        pair: &str,
        order_id: &OrderId,
    ) -> Result<(), Box<dyn std::error::Error>>;
    #[allow(clippy::ptr_arg)]
    async fn order_status(
        &self,
        pair: &str,
        order_id: &OrderId,
    ) -> Result<OrderStatus, Box<dyn std::error::Error>>;
    async fn get_lending_info(
        &self,
        coin: &str,
    ) -> Result<Option<LendingInfo>, Box<dyn std::error::Error>>;
    async fn get_lending_history(
        &self,
        lending_history: LendingHistory,
    ) -> Result<HashMap<String, f64>, Box<dyn std::error::Error>>;
    async fn submit_lending_offer(
        &self,
        coin: &str,
        size: f64,
    ) -> Result<(), Box<dyn std::error::Error>>;
    fn preferred_solusd_pair(&self) -> &'static str;
}

pub fn exchange_client_new(
    exchange: Exchange,
    exchange_credentials: ExchangeCredentials,
) -> Result<Box<dyn ExchangeClient>, Box<dyn std::error::Error>> {
    let exchange_client: Box<dyn ExchangeClient> = match exchange {
        #[cfg(feature = "binance")]
        Exchange::Binance => Box::new(binance_exchange::new(exchange_credentials)?),
        #[cfg(feature = "binance")]
        Exchange::BinanceUs => Box::new(binance_exchange::new_us(exchange_credentials)?),
        #[cfg(feature = "coinbase")]
        Exchange::Coinbase => Box::new(coinbase_exchange::new(exchange_credentials)?),
        #[cfg(feature = "ftx")]
        Exchange::Ftx => Box::new(ftx_exchange::new(exchange_credentials)?),
        #[cfg(feature = "ftx")]
        Exchange::FtxUs => Box::new(ftx_exchange::new_us(exchange_credentials)?),
        Exchange::Kraken => Box::new(kraken_exchange::new(exchange_credentials)?),
    };
    Ok(exchange_client)
}
