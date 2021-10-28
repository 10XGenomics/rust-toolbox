// Copyright (c) 2021 10X Genomics, Inc. All rights reserved.

// This crate provides some utilities for interacting with the eval_expr crate.
//
// These are idiosyncratic.  In particular, there is a built-in list of function names that
// were selected because they were needed elsewhere.  This list will likely be enlarged.

use evalexpr::{ContextWithMutableFunctions, ContextWithMutableVariables, HashMapContext};
use evalexpr::{Function, Value};
use statrs::distribution::ContinuousCDF;
use string_utils::*;
use vector_utils::bin_member;

// ================================================================================================

// List of known function names.

pub fn evalexpr_function_names() -> Vec<String> {
    let mut x = vec!["beta_cdf".to_string()];
    x.sort();
    x
}

// ================================================================================================

// Return the list of variable names in a node.

pub fn vars_of_node(n: &evalexpr::Node) -> Vec<String> {
    let mut x = Vec::<String>::new();
    for i in n.iter_variable_identifiers() {
        x.push((*i).to_string());
    }
    x
}

// Test if function names in node are known.

pub fn test_functions_in_node(n: &evalexpr::Node) -> Result<(), String> {
    let x = evalexpr_function_names();
    for i in n.iter_function_identifiers() {
        if !bin_member(&x, &(*i).to_string()) {
            return Err(format!("Unknown function name {} in expression.", i));
        }
    }
    Ok(())
}

// ================================================================================================

// Convert a function having one of these forms:
// - fn f(x: f64) -> f64;
// - fn f(x: f64, y: f64) -> f64;
// - fn f(x: f64, y: f64, z: f64) -> f64;
// into an evalexpr::Function.
//
// This could be extended to work for zero variables or four/more.

#[macro_export]
macro_rules! evalexpr_fn1 {
    ($f:expr) => {
        Function::new(|t| {
            if t.is_number() {
                let t = t.as_number().unwrap();
                Ok(Value::from($f(t)))
            } else {
                Ok(Value::from(""))
            }
        })
    };
}

#[macro_export]
macro_rules! evalexpr_fn2 {
    ($f:expr) => {
        Function::new(|t| {
            if t.is_tuple() {
                let t = t.as_tuple().unwrap();
                if t.len() == 2 {
                    let x = &t[0];
                    let y = &t[1];
                    if x.is_number() && y.is_number() {
                        let x = x.as_number().unwrap();
                        let y = y.as_number().unwrap();
                        return Ok(Value::from($f(x, y)));
                    }
                }
            }
            Ok(Value::from(""))
        })
    };
}

#[macro_export]
macro_rules! evalexpr_fn3 {
    ($f:expr) => {
        Function::new(|t| {
            if t.is_tuple() {
                let t = t.as_tuple().unwrap();
                if t.len() == 3 {
                    let x = &t[0];
                    let y = &t[1];
                    let z = &t[2];
                    if x.is_number() && y.is_number() && z.is_number() {
                        let x = x.as_number().unwrap();
                        let y = y.as_number().unwrap();
                        let z = z.as_number().unwrap();
                        return Ok(Value::from($f(x, y, z)));
                    }
                }
            }
            Ok(Value::from(""))
        })
    };
}

// ================================================================================================

// Given a a list of variables and values for them, define an evalexpr::HashMapContext that
// includes these variables, as well as some convenient (but arbitrarily chosen) functions.
// This can then be used to evaluate an expression.
//
// Functions take as input zero or more f64 arguments, and return an f64.  The machine allows them
// to be called on arbitrary strings, but if the strings are not all f64, then the return value is
// null.

pub fn define_evalexpr_context(vars: &Vec<String>, vals: &Vec<String>) -> evalexpr::HashMapContext {
    assert_eq!(vars.len(), vals.len());
    let mut c = HashMapContext::new();

    // Define the variable values.

    for i in 0..vars.len() {
        if vals[i].parse::<f64>().is_ok() {
            c.set_value(vars[i].clone(), evalexpr::Value::Float(vals[i].force_f64()))
                .unwrap();
        } else {
            c.set_value(vars[i].clone(), evalexpr::Value::String(vals[i].clone()))
                .unwrap();
        }
    }

    // Define the beta cdf function.
    //
    // Requirements (not tested, but out of range should return *some* value):
    // - 0 <= x <= 1
    // - a > 0
    // - b > 0.

    fn beta_cdf(x: f64, a: f64, b: f64) -> f64 {
        let n = statrs::distribution::Beta::new(a, b).unwrap();
        n.cdf(x)
    }
    c.set_function("beta_cdf".to_string(), evalexpr_fn3![beta_cdf])
        .unwrap();

    // Done.

    c
}
