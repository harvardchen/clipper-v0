// automatically generated by rust-bindgen

#![allow(dead_code)]
extern crate libc;
pub mod common;

use libc::{c_int, c_double, c_char, c_uint};
// use common;
use std::mem;
use std::clone::Clone;


#[allow(non_camel_case_types)]
pub type Struct_svm_node = common::Struct_feature_node;

#[repr(C)]
// #[derive(Copy, Debug)]
// #[allow(raw_pointer_derive)]
pub struct Struct_svm_problem {
    pub l: c_int,
    pub y: *const c_double,
    pub x: *const *const Struct_svm_node,
}
// impl ::std::clone::Clone for Struct_svm_problem {
//     fn clone(&self) -> Self { *self }
// }
// impl ::std::default::Default for Struct_svm_problem {
//     fn default() -> Self { unsafe { ::std::mem::zeroed() } }
// }
// pub type Enum_Unnamed1 = c_uint;
pub const C_SVC: c_int = 0;
pub const NU_SVC: c_int = 1;
pub const ONE_CLASS: c_int = 2;
pub const EPSILON_SVR: c_int = 3;
pub const NU_SVR: c_uint = 4;
// pub type Enum_Unnamed2 = c_uint;
pub const LINEAR: c_int = 0;
pub const POLY: c_int = 1;
pub const RBF: c_int = 2;
pub const SIGMOID: c_int = 3;
pub const PRECOMPUTED: c_int = 4;
#[repr(C)]
#[derive(Copy, Debug)]
#[allow(non_snake_case)]
pub struct Struct_svm_parameter {
    pub svm_type: c_int,
    pub kernel_type: c_int,
    pub degree: c_int,
    pub gamma: c_double,
    pub coef0: c_double,
    pub cache_size: c_double,
    pub eps: c_double,
    pub C: c_double,
    pub nr_weight: c_int,
    pub weight_label: *const c_int,
    pub weight: *const c_double,
    pub nu: c_double,
    pub p: c_double,
    pub shrinking: c_int,
    pub probability: c_int,
}
impl Clone for Struct_svm_parameter {
    fn clone(&self) -> Self {
        *self
    }
}
impl ::std::default::Default for Struct_svm_parameter {
    fn default() -> Self {
        unsafe { mem::zeroed() }
    }
}
#[repr(C)]
// #[derive(Copy, Debug)]
// #[derive(Debug)]
#[allow(non_snake_case)]
pub struct Struct_svm_model {
    pub param: Struct_svm_parameter,
    pub nr_class: c_int,
    pub l: c_int,
    pub SV: *const *const Struct_svm_node,
    pub sv_coef: *const *const c_double,
    pub rho: *const c_double,
    pub probA: *const c_double,
    pub probB: *const c_double,
    pub sv_indices: *const c_int,
    pub label: *const c_int,
    pub nSV: *const c_int,
    pub free_sv: c_int,
}
// impl ::std::clone::Clone for Struct_svm_model {
//     fn clone(&self) -> Self { *self }
// }
impl ::std::default::Default for Struct_svm_model {
    fn default() -> Self {
        unsafe { mem::zeroed() }
    }
}
// #[link(name = "svm")]
extern "C" {
    pub static mut libsvm_version: c_int;
}
// #[link(name = "svm")]
extern "C" {
    pub fn svm_train(prob: *const Struct_svm_problem,
                     param: *const Struct_svm_parameter)
                     -> *const Struct_svm_model;
    pub fn svm_cross_validation(prob: *const Struct_svm_problem,
                                param: *const Struct_svm_parameter,
                                nr_fold: c_int,
                                target: *const c_double)
                                -> ();
    pub fn svm_save_model(model_file_name: *const c_char, model: *const Struct_svm_model) -> c_int;
    pub fn svm_load_model(model_file_name: *const c_char) -> *const Struct_svm_model;
    pub fn svm_get_svm_type(model: *const Struct_svm_model) -> c_int;
    pub fn svm_get_nr_class(model: *const Struct_svm_model) -> c_int;
    pub fn svm_get_labels(model: *const Struct_svm_model, label: *const c_int) -> ();
    pub fn svm_get_sv_indices(model: *const Struct_svm_model, sv_indices: *const c_int) -> ();
    pub fn svm_get_nr_sv(model: *const Struct_svm_model) -> c_int;
    pub fn svm_get_svr_probability(model: *const Struct_svm_model) -> c_double;
    pub fn svm_predict_values(model: *const Struct_svm_model,
                              x: *const Struct_svm_node,
                              dec_values: *const c_double)
                              -> c_double;
    pub fn svm_predict(model: *const Struct_svm_model, x: *const Struct_svm_node) -> c_double;
    pub fn svm_predict_probability(model: *const Struct_svm_model,
                                   x: *const Struct_svm_node,
                                   prob_estimates: *const c_double)
                                   -> c_double;
    pub fn svm_free_model_content(model_ptr: *mut Struct_svm_model) -> ();
    pub fn svm_free_and_destroy_model(model_ptr_ptr: *mut *mut Struct_svm_model) -> ();
    pub fn svm_destroy_param(param: *mut Struct_svm_parameter) -> ();
    pub fn svm_check_parameter(prob: *const Struct_svm_problem,
                               param: *const Struct_svm_parameter)
                               -> *const c_char;
    pub fn svm_check_probability_model(model: *const Struct_svm_model) -> c_int;
}


#[cfg(test)]
mod tests {
    use super::*;
    extern crate rand;
    use self::rand::{thread_rng, Rng};
    use std::{ptr, slice};
    use std::marker::PhantomData;
    // use common;

    fn random_floats(d: usize) -> Vec<f64> {
        let mut rng = thread_rng();
        rng.gen_iter::<f64>().take(d).collect::<Vec<f64>>()
    }

    #[test]
    fn train_model() {
        let num_examples = 10;
        let train_data = (0..num_examples)
                             .map(|_| random_floats(8))
                             .collect::<Vec<Vec<f64>>>();
        let mut labels = Vec::with_capacity(num_examples);
        for i in 0..num_examples {
            if i % 2 == 0 {
                labels.push(1.0);
            } else {
                // TODO: should these be 0.0 or -1.0???
                labels.push(0.0);
            }
        }
        let problem = SVMProblem::from_training_data(&train_data, &labels);
        let params = Struct_svm_parameter {
            svm_type: C_SVC,
            kernel_type: 2,
            degree: 3,
            gamma: 0.1_f64, // sklearn default
            coef0: 0.0,
            cache_size: 200_f64,
            eps: 0.001,
            C: 1_f64,
            nr_weight: 0,
            weight_label: ptr::null_mut(),
            weight: ptr::null_mut(),
            nu: 0.5,
            p: 0.1,
            shrinking: 1,
            probability: 0,
        };
        unsafe {
            let m = svm_train(&problem.raw as *const Struct_svm_problem,
                              &params as *const Struct_svm_parameter);
            let nSV = slice::from_raw_parts((*m).nSV, (*m).nr_class as usize).to_vec();
            // TODO: this assert will fail, just want to see what nSV is
            assert_eq!(nSV, vec![0]);
        }
    }

    pub struct SVMProblem {
        pub num_examples: i32,
        pub max_index: i32,
        pub labels: Vec<f64>,
        pub examples: Vec<Vec<common::Struct_feature_node>>,
        pub example_ptrs: Vec<*const common::Struct_feature_node>,
        pub bias: f64,
        pub raw: Struct_svm_problem,
    }

    impl SVMProblem {
        pub fn from_training_data(xs: &Vec<Vec<f64>>, ys: &Vec<f64>) -> SVMProblem {
            let (examples, max_index) = make_sparse_matrix(xs);
            let example_ptrs = vec_to_ptrs(&examples).vec;
            let labels = ys.clone();
            let raw = Struct_svm_problem {
                l: ys.len() as i32,
                y: labels.as_ptr(),
                x: (&example_ptrs[..]).as_ptr(),
            };
            SVMProblem {
                num_examples: ys.len() as i32,
                max_index: max_index,
                labels: labels,
                examples: examples,
                example_ptrs: example_ptrs,
                bias: -1.0,
                raw: raw,
            }
        }
    }

    pub struct PtrVec<'a, T: 'a> {
        pub vec: Vec<*const T>,
        phantom: PhantomData<&'a Vec<T>>,
    }

    // TODO figure out how to use lifetimes
    pub fn vec_to_ptrs<'a, T>(examples: &'a Vec<Vec<T>>) -> PtrVec<'a, T> {

        // let all_x_vec: Vec<*mut Struct_feature_node> = Vec::new();
        let mut first_x_vec: Vec<*const T> = Vec::with_capacity(examples.len());

        for i in 0..examples.len() {
            first_x_vec.push((&examples[i][..]).as_ptr());
        }
        PtrVec {
            vec: first_x_vec,
            phantom: PhantomData,
        }
    }

    pub fn make_sparse_matrix(xs: &Vec<Vec<f64>>) -> (Vec<Vec<common::Struct_feature_node>>, i32) {

        let mut examples: Vec<Vec<common::Struct_feature_node>> = Vec::with_capacity(xs.len());

        let mut max_index = 1;
        for example in xs {
            let mut features: Vec<common::Struct_feature_node> = Vec::new();
            let mut idx = 1; // liblinear is 1-based indexing
            for f in example.iter() {
                if *f != 0.0 {
                    if idx > max_index {
                        max_index = idx;
                    }
                    let f_node = common::Struct_feature_node {
                        index: idx,
                        value: *f,
                    };
                    features.push(f_node);
                }
                idx += 1;
            }
            features.push(common::Struct_feature_node {
                index: -1,
                value: 0.0,
            }); // -1 indicates end of feature vector
            examples.push(features);
        }
        (examples, max_index)
    }
}