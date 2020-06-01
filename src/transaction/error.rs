// Copyright 2015-2020 Parity Technologies (UK) Ltd.
// This file is part of Open Ethereum.

// Open Ethereum is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Open Ethereum is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Open Ethereum.  If not, see <http://www.gnu.org/licenses/>.

use std::{fmt, error};

use ethereum_types::U256;
use parity_crypto::publickey::{Error as EthPublicKeyCryptoError};
use rlp;
use unexpected::OutOfBounds;

use errors::ExecutionError;

#[derive(Debug, PartialEq, Clone)]
/// Errors concerning transaction processing.
pub enum Error {
	/// Transaction is already imported to the queue
	AlreadyImported,
	/// Transaction is not valid anymore (state already has higher nonce)
	Old,
	/// Transaction was not imported to the queue because limit has been reached.
	LimitReached,
	/// Transaction's gas price is below threshold.
	InsufficientGasPrice {
		/// Minimal expected gas price
		minimal: U256,
		/// Transaction gas price
		got: U256,
	},
	/// Transaction has too low fee
	/// (there is already a transaction with the same sender-nonce but higher gas price)
	TooCheapToReplace {
		/// previous transaction's gas price
		prev: Option<U256>,
		/// new transaction's gas price
		new: Option<U256>,
	},
	/// Transaction's gas is below currently set minimal gas requirement.
	InsufficientGas {
		/// Minimal expected gas
		minimal: U256,
		/// Transaction gas
		got: U256,
	},
	/// Sender doesn't have enough funds to pay for this transaction
	InsufficientBalance {
		/// Senders balance
		balance: U256,
		/// Transaction cost
		cost: U256,
	},
	/// Transactions gas is higher then current gas limit
	GasLimitExceeded {
		/// Current gas limit
		limit: U256,
		/// Declared transaction gas
		got: U256,
	},
	/// Transaction's gas limit (aka gas) is invalid.
	InvalidGasLimit(OutOfBounds<U256>),
	/// Transaction sender is banned.
	SenderBanned,
	/// Transaction receipient is banned.
	RecipientBanned,
	/// Contract creation code is banned.
	CodeBanned,
	/// Invalid chain ID given.
	InvalidChainId,
	/// Not enough permissions given by permission contract.
	NotAllowed,
	/// Signature error
	InvalidSignature(String),
	/// Transaction too big
	TooBig,
	/// Invalid RLP encoding
	InvalidRlp(String),
}

impl From<EthPublicKeyCryptoError> for Error {
	fn from(err: EthPublicKeyCryptoError) -> Self {
		Error::InvalidSignature(format!("{}", err))
	}
}

impl From<rlp::DecoderError> for Error {
	fn from(err: rlp::DecoderError) -> Self {
		Error::InvalidRlp(format!("{}", err))
	}
}

impl fmt::Display for Error {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		use self::Error::*;
		let msg = match *self {
			AlreadyImported => "Already imported".into(),
			Old => "No longer valid".into(),
			TooCheapToReplace { prev, new } =>
				format!("Gas price too low to replace, previous tx gas: {:?}, new tx gas: {:?}",
						prev, new
				),
			LimitReached => "Transaction limit reached".into(),
			InsufficientGasPrice { minimal, got } =>
				format!("Insufficient gas price. Min={}, Given={}", minimal, got),
			InsufficientGas { minimal, got } =>
				format!("Insufficient gas. Min={}, Given={}", minimal, got),
			InsufficientBalance { balance, cost } =>
				format!("Insufficient balance for transaction. Balance={}, Cost={}",
					balance, cost),
			GasLimitExceeded { limit, got } =>
				format!("Gas limit exceeded. Limit={}, Given={}", limit, got),
			InvalidGasLimit(ref err) => format!("Invalid gas limit. {}", err),
			SenderBanned => "Sender is temporarily banned.".into(),
			RecipientBanned => "Recipient is temporarily banned.".into(),
			CodeBanned => "Contract code is temporarily banned.".into(),
			InvalidChainId => "Transaction of this chain ID is not allowed on this chain.".into(),
			InvalidSignature(ref err) => format!("Transaction has invalid signature: {}.", err),
			NotAllowed => "Sender does not have permissions to execute this type of transaction".into(),
			TooBig => "Transaction too big".into(),
			InvalidRlp(ref err) => format!("Transaction has invalid RLP structure: {}.", err),
		};

		f.write_fmt(format_args!("Transaction error ({})", msg))
	}
}

impl error::Error for Error {
	fn description(&self) -> &str {
		"Transaction error"
	}
}

/// VM errors.
#[derive(Debug, Clone, PartialEq)]
pub enum VmError {
	/// `OutOfGas` is returned when transaction execution runs out of gas.
	/// The state should be reverted to the state from before the
	/// transaction execution. But it does not mean that transaction
	/// was invalid. Balance still should be transfered and nonce
	/// should be increased.
	OutOfGas,
	/// `BadJumpDestination` is returned when execution tried to move
	/// to position that wasn't marked with JUMPDEST instruction
	BadJumpDestination {
		/// Position the code tried to jump to.
		destination: usize
	},
	/// `BadInstructions` is returned when given instruction is not supported
	BadInstruction {
		/// Unrecognized opcode
		instruction: u8,
	},
	/// `StackUnderflow` when there is not enough stack elements to execute instruction
	StackUnderflow {
		/// Invoked instruction
		instruction: &'static str,
		/// How many stack elements was requested by instruction
		wanted: usize,
		/// How many elements were on stack
		on_stack: usize
	},
	/// When execution would exceed defined Stack Limit
	OutOfStack {
		/// Invoked instruction
		instruction: &'static str,
		/// How many stack elements instruction wanted to push
		wanted: usize,
		/// What was the stack limit
		limit: usize
	},
	/// Built-in contract failed on given input
	BuiltIn(&'static str),
	/// When execution tries to modify the state in static context
	MutableCallInStaticContext,
	/// Likely to cause consensus issues.
	Internal(String),
	/// Wasm runtime error
	Wasm(String),
	/// Out of bounds access in RETURNDATACOPY.
	OutOfBounds,
	/// Execution has been reverted with REVERT.
	Reverted,
}


impl fmt::Display for VmError {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		use self::VmError::*;
		match *self {
			OutOfGas => write!(f, "Out of gas"),
			BadJumpDestination { destination } => write!(f, "Bad jump destination {:x}", destination),
			BadInstruction { instruction } => write!(f, "Bad instruction {:x}",  instruction),
			StackUnderflow { instruction, wanted, on_stack } => write!(f, "Stack underflow {} {}/{}", instruction, wanted, on_stack),
			OutOfStack { instruction, wanted, limit } => write!(f, "Out of stack {} {}/{}", instruction, wanted, limit),
			BuiltIn(name) => write!(f, "Built-in failed: {}", name),
			Internal(ref msg) => write!(f, "Internal error: {}", msg),
			MutableCallInStaticContext => write!(f, "Mutable call in static context"),
			Wasm(ref msg) => write!(f, "Internal error: {}", msg),
			OutOfBounds => write!(f, "Out of bounds"),
			Reverted => write!(f, "Reverted"),
		}
	}
}


/// Result of executing the transaction.
#[derive(PartialEq, Debug, Clone)]
pub enum CallError {
	/// Couldn't find the transaction in the chain.
	TransactionNotFound,
	/// Couldn't find requested block's state in the chain.
	StatePruned,
	/// Couldn't find an amount of gas that didn't result in an exception.
	Exceptional(VmError),
	/// Corrupt state.
	StateCorrupt,
	/// Error executing.
	Execution(ExecutionError),
}

impl From<ExecutionError> for CallError {
	fn from(error: ExecutionError) -> Self {
		CallError::Execution(error)
	}
}

impl fmt::Display for CallError {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		use self::CallError::*;
		let msg = match *self {
			TransactionNotFound => "Transaction couldn't be found in the chain".into(),
			StatePruned => "Couldn't find the transaction block's state in the chain".into(),
			Exceptional(ref e) => format!("An exception ({}) happened in the execution", e),
			StateCorrupt => "Stored state found to be corrupted.".into(),
			Execution(ref e) => format!("{}", e),
		};

		f.write_fmt(format_args!("Transaction execution error ({}).", msg))
	}
}