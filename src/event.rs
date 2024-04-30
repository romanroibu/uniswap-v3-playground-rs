use rust_decimal::Decimal;
use web3::ethabi::Address;

#[derive(Debug)]
pub struct SwapEvent {
	pub sender: Address,
	pub receiver: Address,
	pub direction: SwapDirection,
	pub amounts: SwapAmounts,
}

#[derive(Debug)]
pub enum SwapDirection {
	DaiToUsdc,
	UsdcToDai,
}

#[derive(Debug)]
pub struct SwapAmounts {
	pub dai: Decimal,
	pub usdc: Decimal,
}
