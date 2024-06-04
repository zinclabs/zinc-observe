// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.  See the NOTICE file
// distributed with this work for additional information
// regarding copyright ownership.  The ASF licenses this file
// to you under the Apache License, Version 2.0 (the
// "License"); you may not use this file except in compliance
// with the License.  You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing,
// software distributed under the License is distributed on an
// "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied.  See the License for the
// specific language governing permissions and limitations
// under the License.

/// Creates external API functions for an array UDF. Specifically, creates
///
/// 1. Single `ScalarUDF` instance
///
/// Creates a singleton `ScalarUDF` of the `$UDF` function named `STATIC_$(UDF)` and a
/// function named `$SCALAR_UDF_FUNC` which returns that function named `STATIC_$(UDF)`.
///
/// This is used to ensure creating the list of `ScalarUDF` only happens once.
///
/// # 2. `expr_fn` style function
///
/// These are functions that create an `Expr` that invokes the UDF, used
/// primarily to programmatically create expressions.
///
/// For example:
/// ```text
/// pub fn array_to_string(delimiter: Expr) -> Expr {
/// ...
/// }
/// ```
/// # Arguments
/// * `UDF`: name of the [`ScalarUDFImpl`]
/// * `EXPR_FN`: name of the expr_fn function to be created
/// * `arg`: 0 or more named arguments for the function
/// * `DOC`: documentation string for the function
/// * `SCALAR_UDF_FUNC`: name of the function to create (just) the `ScalarUDF`
///
/// [`ScalarUDFImpl`]: datafusion_expr::ScalarUDFImpl
macro_rules! make_udf_expr_and_func {
    ($UDF:ty, $EXPR_FN:ident, $($arg:ident)*, $DOC:expr , $SCALAR_UDF_FN:ident) => {
        paste::paste! {
            // "fluent expr_fn" style function
            #[doc = $DOC]
            pub fn $EXPR_FN($($arg: datafusion_expr::Expr),*) -> datafusion_expr::Expr {
                datafusion_expr::Expr::ScalarFunction(datafusion_expr::expr::ScalarFunction::new_udf(
                    $SCALAR_UDF_FN(),
                    vec![$($arg),*],
                ))
            }
            create_func!($UDF, $SCALAR_UDF_FN);
        }
    };
    ($UDF:ty, $EXPR_FN:ident, $DOC:expr , $SCALAR_UDF_FN:ident) => {
        paste::paste! {
            // "fluent expr_fn" style function
            #[doc = $DOC]
            pub fn $EXPR_FN(arg: Vec<datafusion_expr::Expr>) -> datafusion_expr::Expr {
                datafusion_expr::Expr::ScalarFunction(datafusion_expr::expr::ScalarFunction::new_udf(
                    $SCALAR_UDF_FN(),
                    arg,
                ))
            }
            create_func!($UDF, $SCALAR_UDF_FN);
        }
    };
}

/// Creates a singleton `ScalarUDF` of the `$UDF` function named `STATIC_$(UDF)` and a
/// function named `$SCALAR_UDF_FUNC` which returns that function named `STATIC_$(UDF)`.
///
/// This is used to ensure creating the list of `ScalarUDF` only happens once.
///
/// # Arguments
/// * `UDF`: name of the [`ScalarUDFImpl`]
/// * `SCALAR_UDF_FUNC`: name of the function to create (just) the `ScalarUDF`
///
/// [`ScalarUDFImpl`]: datafusion_expr::ScalarUDFImpl
macro_rules! create_func {
    ($UDF:ty, $SCALAR_UDF_FN:ident) => {
        paste::paste! {
            /// Singleton instance of [`$UDF`], ensures the UDF is only created once
            /// named STATIC_$(UDF). For example `STATIC_ArrayToString`
            #[allow(non_upper_case_globals)]
            static [< STATIC_ $UDF >]: std::sync::OnceLock<std::sync::Arc<datafusion_expr::ScalarUDF>> =
                std::sync::OnceLock::new();
            /// ScalarFunction that returns a [`ScalarUDF`] for [`$UDF`]
            ///
            /// [`ScalarUDF`]: datafusion_expr::ScalarUDF
            pub fn $SCALAR_UDF_FN() -> std::sync::Arc<datafusion_expr::ScalarUDF> {
                [< STATIC_ $UDF >]
                    .get_or_init(|| {
                        std::sync::Arc::new(datafusion_expr::ScalarUDF::new_from_impl(
                            <$UDF>::new(),
                        ))
                    })
                    .clone()
            }
        }
    };
}
