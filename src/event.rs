use rust_decimal::Decimal;
use web3::ethabi::Address;

#[derive(Debug, PartialEq)]
pub struct SwapEvent {
	pub sender: Address,
	pub receiver: Address,
	pub direction: SwapDirection,
	pub amounts: SwapAmounts,
}

#[derive(Debug, PartialEq)]
pub enum SwapDirection {
	DaiToUsdc,
	UsdcToDai,
}

#[derive(Debug, PartialEq)]
pub struct SwapAmounts {
	pub dai: Decimal,
	pub usdc: Decimal,
}

impl SwapAmounts {
	pub(crate) fn abs(&self) -> SwapAmounts {
		SwapAmounts { dai: self.dai.abs(), usdc: self.usdc.abs() }
	}
}
