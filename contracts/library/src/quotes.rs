use soroban_sdk::{Address, Env, Vec};

use crate::reserves::{get_reserves};
use crate::error::LibraryError;

/// given some amount of an asset and pair reserves, returns an equivalent amount of the other asset
pub fn quote(amount_a: i128, reserve_a: i128, reserve_b: i128) -> Result<i128, LibraryError> {
    if amount_a <= 0 {
        return Err(LibraryError::InsufficientAmount);
    }
    if reserve_a <= 0 || reserve_b <= 0 {
        return Err(LibraryError::InsufficientLiquidity);
    }
    Ok(amount_a.checked_mul(reserve_b).ok_or(LibraryError::InsufficientLiquidity)?.checked_div(reserve_a).ok_or(LibraryError::InsufficientLiquidity)?)
}

/// given an input amount of an asset and pair reserves, returns the maximum output amount of the other asset
pub fn get_amount_out(amount_in: i128, reserve_in: i128, reserve_out: i128) -> Result<i128, LibraryError> {
    if amount_in <= 0 {
        return Err(LibraryError::InsufficientInputAmount);
    }
    if reserve_in <= 0 || reserve_out <= 0 {
        return Err(LibraryError::InsufficientLiquidity);
    }

    let amount_in_with_fee = amount_in.checked_mul(997).unwrap();
    let numerator = amount_in_with_fee.checked_mul(reserve_out).unwrap();

    let denominator = reserve_in.checked_mul(1000).unwrap().checked_add(amount_in_with_fee).unwrap();

    Ok(numerator.checked_div(denominator).unwrap())
}

/// given an output amount of an asset and pair reserves, returns a required input amount of the other asset
pub fn get_amount_in(amount_out: i128, reserve_in: i128, reserve_out: i128) -> Result<i128, LibraryError> {
    if amount_out <= 0 {
        return Err(LibraryError::InsufficientOutputAmount);
    }
    if reserve_in <= 0 || reserve_out <= 0 {
        return Err(LibraryError::InsufficientLiquidity);
    }
    let numerator = reserve_in.checked_mul(amount_out).unwrap().checked_mul(1000).unwrap();
    let denominator = reserve_out.checked_sub(amount_out).unwrap().checked_mul(997).unwrap();
    Ok(numerator.checked_div(denominator).unwrap().checked_add(1).unwrap())
}

/// performs chained getAmountOut calculations on any number of pairs 
pub fn get_amounts_out(e: Env, factory: Address, amount_in: i128, path: Vec<Address>) -> Result<Vec<i128>, LibraryError> {
    if path.len() < 2 {
        return Err(LibraryError::InvalidPath);
    }

    let mut amounts = Vec::new(&e);
    amounts.push_back(amount_in);

    for i in 0..path.len() - 1 {
        let (reserve_in, reserve_out) = get_reserves(e.clone(), factory.clone(), path.get(i).unwrap(), path.get(i+1).unwrap())?;
        amounts.push_back(get_amount_out(amounts.get(i).unwrap(), reserve_in, reserve_out)?);
    }

    Ok(amounts)
}

/// performs chained getAmountIn calculations on any number of pairs
pub fn get_amounts_in(e: Env, factory: Address, amount_out: i128, path: Vec<Address>) -> Result<Vec<i128>, LibraryError> {
    if path.len() < 2 {
        return Err(LibraryError::InvalidPath);
    }

    let mut amounts = Vec::new(&e);
    amounts.push_front(amount_out);

    for i in (1..path.len()).rev() {
        let (reserve_in, reserve_out) = get_reserves(e.clone(), factory.clone(), path.get(i-1).unwrap(), path.get(i).unwrap())?;
        let new_amount = get_amount_in(amounts.get(0).unwrap(), reserve_in, reserve_out)?;
        amounts.push_front(new_amount);
    }

    Ok(amounts)
}
