use anyhow::{anyhow, Context, Result};
use rust_decimal::{prelude::ToPrimitive, Decimal};
use web3::{
	ethabi::{Address, Int, LogParam, Token},
	types::U256,
};

use crate::event::{SwapAmounts, SwapDirection, SwapEvent};

pub(crate) struct SwapParser;

macro_rules! type_err {
	($actual:literal, $expected:literal, $name:expr) => {
		Err(anyhow!("Expected log param '{}' of type '{}' but got '{}'", $name, $expected, $actual))
	};
}

impl SwapParser {
	const DECIMAL_PRECISION: u32 = 2;
	const DAI_BASE: u32 = 18;
	const USDC_BASE: u32 = 6;

	pub(crate) fn parse(log: web3::types::Log, abi: &web3::ethabi::Event) -> Result<SwapEvent> {
		let log = web3::ethabi::RawLog { topics: log.topics, data: log.data.0 };
		let log = &abi.parse_log(log)?;

		let sender = Self::get_address(log, "sender")?;
		let receiver = Self::get_address(log, "recipient")?;
		let dai = Self::get_int(log, "amount0")?;
		let usdc = Self::get_int(log, "amount1")?;

		let amounts = SwapAmounts {
			dai: Self::to_decimal(dai, Self::DAI_BASE),
			usdc: Self::to_decimal(usdc, Self::USDC_BASE),
		};

		let event = SwapEvent {
			sender,
			receiver,
			direction: Self::get_direction(&amounts)?,
			amounts: amounts.abs(),
		};

		Ok(event)
	}

	fn get_direction(amounts: &SwapAmounts) -> Result<SwapDirection> {
		let dai_pos = amounts.dai.is_sign_positive();
		let usdc_pos = amounts.usdc.is_sign_positive();

		match (dai_pos, usdc_pos) {
			(true, false) => Ok(SwapDirection::DaiToUsdc),
			(false, true) => Ok(SwapDirection::UsdcToDai),
			(true, true) =>
				Err(anyhow!("Swap amounts must have distinct signs, but both are positive")),
			(false, false) =>
				Err(anyhow!("Swap amounts must have distinct signs, but both are negative")),
		}
	}

	fn get_address<'a>(log: &'a web3::ethabi::Log, name: &'static str) -> Result<Address> {
		match Self::get_param(log, name)?.value {
			Token::Address(address) => Ok(address),
			Token::FixedBytes(_) => type_err!("FixedBytes", "Address", name),
			Token::Bytes(_) => type_err!("Bytes", "Address", name),
			Token::Int(_) => type_err!("Int", "Address", name),
			Token::Uint(_) => type_err!("Uint", "Address", name),
			Token::Bool(_) => type_err!("Bool", "Address", name),
			Token::String(_) => type_err!("String", "Address", name),
			Token::FixedArray(_) => type_err!("FixedArray", "Address", name),
			Token::Array(_) => type_err!("Array", "Address", name),
			Token::Tuple(_) => type_err!("Tuple", "Address", name),
		}
	}

	fn get_int<'a>(log: &'a web3::ethabi::Log, name: &'static str) -> Result<Int> {
		match Self::get_param(log, name)?.value {
			Token::Int(int) => Ok(int),
			Token::Address(_) => type_err!("Address", "Int", name),
			Token::FixedBytes(_) => type_err!("FixedBytes", "Int", name),
			Token::Bytes(_) => type_err!("Bytes", "Int", name),
			Token::Uint(_) => type_err!("Uint", "Int", name),
			Token::Bool(_) => type_err!("Bool", "Int", name),
			Token::String(_) => type_err!("String", "Int", name),
			Token::FixedArray(_) => type_err!("FixedArray", "Int", name),
			Token::Array(_) => type_err!("Array", "Int", name),
			Token::Tuple(_) => type_err!("Tuple", "Int", name),
		}
	}

	fn get_param<'a>(log: &'a web3::ethabi::Log, name: &'static str) -> Result<&'a LogParam> {
		log.params
			.iter()
			.find(|p| p.name == name)
			.with_context(|| format!("Missing log param '{}'", name))
	}

	fn to_decimal(n: U256, base: u32) -> Decimal {
		let dp = Self::DECIMAL_PRECISION;

		let base = base - dp;
		let base = U256::from(10).pow(U256::from(base));

		let is_negative = n > U256::from(u128::MAX);

		let n = if is_negative { U256::MAX - n } else { n };

		let n: U256 = n / base;
		let n = n.as_u128().to_i128().unwrap();
		let n = if is_negative { n * -1 } else { n };

		let n = Decimal::from_i128_with_scale(n, dp);
		n
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	mod get_direction {
		use super::*;

		mod ok {
			use super::*;

			#[test]
			fn dai_to_usdc() {
				let dai = Decimal::new(12345, 2);
				let usdc = Decimal::new(-678, 2);
				let amounts = SwapAmounts { dai, usdc };
				let result = SwapParser::get_direction(&amounts);

				assert!(result.is_ok());
				assert_eq!(result.unwrap(), SwapDirection::DaiToUsdc);
			}

			#[test]
			fn usdc_to_dai() {
				let dai = Decimal::new(-1234, 2);
				let usdc = Decimal::new(6678, 2);
				let amounts = SwapAmounts { dai, usdc };
				let result = SwapParser::get_direction(&amounts);

				assert!(result.is_ok());
				assert_eq!(result.unwrap(), SwapDirection::UsdcToDai);
			}
		}

		mod err {
			use super::*;

			#[test]
			fn both_positive() {
				let dai = Decimal::new(12345, 2);
				let usdc = Decimal::new(6789, 2);
				let amounts = SwapAmounts { dai, usdc };
				let result = SwapParser::get_direction(&amounts);

				assert!(result.is_err());
				assert_eq!(
					result.unwrap_err().to_string(),
					"Swap amounts must have distinct signs, but both are positive".to_string()
				);
			}

			#[test]
			fn both_negative() {
				let dai = Decimal::new(-1234, 2);
				let usdc = Decimal::new(-567, 2);
				let amounts = SwapAmounts { dai, usdc };
				let result = SwapParser::get_direction(&amounts);

				assert!(result.is_err());
				assert_eq!(
					result.unwrap_err().to_string(),
					"Swap amounts must have distinct signs, but both are negative".to_string()
				);
			}
		}
	}

	mod get_address {
		use super::*;

		#[test]
		fn ok() {
			let address = web3::types::H160([123; 20]);
			let log = web3::ethabi::Log {
				params: vec![LogParam { name: "foo".to_string(), value: Token::Address(address) }],
			};
			let result = SwapParser::get_address(&log, "foo");

			assert!(result.is_ok());
			assert_eq!(result.unwrap(), address);
		}

		mod err {
			use super::*;

			#[test]
			fn missing() {
				let address = web3::types::H160([123; 20]);
				let log = web3::ethabi::Log {
					params: vec![LogParam {
						name: "bar".to_string(),
						value: Token::Address(address),
					}],
				};
				let result = SwapParser::get_address(&log, "foo");

				assert!(result.is_err());
				assert_eq!(result.unwrap_err().to_string(), "Missing log param 'foo'".to_string());
			}

			#[test]
			fn unexpected_type() {
				let int = U256::from_little_endian(&[123; 20]);
				let log = web3::ethabi::Log {
					params: vec![LogParam { name: "foo".to_string(), value: Token::Int(int) }],
				};
				let result = SwapParser::get_address(&log, "foo");

				assert!(result.is_err());
				assert_eq!(
					result.unwrap_err().to_string(),
					"Expected log param 'foo' of type 'Address' but got 'Int'".to_string()
				);
			}
		}
	}

	mod get_int {
		use super::*;

		#[test]
		fn ok() {
			let int = U256::from_little_endian(&[123; 20]);
			let log = web3::ethabi::Log {
				params: vec![LogParam { name: "foo".to_string(), value: Token::Int(int) }],
			};
			let result = SwapParser::get_int(&log, "foo");

			assert!(result.is_ok());
			assert_eq!(result.unwrap(), int);
		}

		mod err {
			use super::*;

			#[test]
			fn missing() {
				let int = U256::from_little_endian(&[123; 20]);
				let log = web3::ethabi::Log {
					params: vec![LogParam { name: "bar".to_string(), value: Token::Int(int) }],
				};
				let result = SwapParser::get_int(&log, "foo");

				assert!(result.is_err());
				assert_eq!(result.unwrap_err().to_string(), "Missing log param 'foo'".to_string());
			}

			#[test]
			fn unexpected_type() {
				let address = web3::types::H160([123; 20]);
				let log = web3::ethabi::Log {
					params: vec![LogParam {
						name: "foo".to_string(),
						value: Token::Address(address),
					}],
				};
				let result = SwapParser::get_int(&log, "foo");

				assert!(result.is_err());
				assert_eq!(
					result.unwrap_err().to_string(),
					"Expected log param 'foo' of type 'Int' but got 'Address'".to_string()
				);
			}
		}
	}

	mod to_decimal {
		use super::*;

		#[test]
		fn positive() {
			let dai_int = U256::from_dec_str("15851874999999999770624").unwrap();
			let dai_dec = Decimal::new(1585187, SwapParser::DECIMAL_PRECISION);

			assert_eq!(dai_dec, SwapParser::to_decimal(dai_int, SwapParser::DAI_BASE));
		}

		#[test]
		fn negative() {
			let usdc_int = U256::from_dec_str(
				"115792089237316195423570985008687907853269984665640564039457584007897279268723",
			)
			.unwrap();
			let usdc_dec = Decimal::new(-1585037, SwapParser::DECIMAL_PRECISION);

			assert_eq!(usdc_dec, SwapParser::to_decimal(usdc_int, SwapParser::USDC_BASE));
		}
	}
}
