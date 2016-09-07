#![allow(dead_code, unused_variables)]
use std::net::TcpStream;
use byteorder::{LittleEndian, WriteBytesExt, ReadBytesExt};
use std::io::{Read, Write, Cursor};
use std::mem;
use server::{Input, Output, InputType};
use batching::RpcPredictRequest;
use lz4::{EncoderBuilder, Decoder};
use std::error::Error;




/// Clipper supports 4 input formats: Strings, Ints, Floats, and Bytes.
///
///
///
///
///
/// #RPC HEADER format
/// 1 byte: input format:
///     1. fixed length i32
///     2. fixed length f64
///     3. fixed length u8
///     4. variable length i32
///     5. variable length f64
///     6. variable length u8
///     7. variable length utf-8 encoded string
///
/// _4 bytes: the number of inputs as u32_
///
/// Format specific headers:
/// fixed length formats {1,2,3}:
/// _4 bytes: input length_
/// The input contents sequentially
///
/// Variable length numeric inputs:
/// _4 bytes_: number of content bytes (so we know how much to recieve)
/// Each input has the format
/// <4 bytes for the input length, followed by the input>
///
/// Strings:
/// _4 bytes_: number of content bytes (so we know how much to recieve)
/// A list of 4 byte i32s indicating the length of each individual string when uncompressed,
/// followed by all of the strings concatenated together and compressed
/// with LZ4

const SHUTDOWN_CODE: u8 = 0;
const FIXEDINT_CODE: u8 = 1;
const FIXEDFLOAT_CODE: u8 = 2;
const FIXEDBYTE_CODE: u8 = 3;
const VARINT_CODE: u8 = 4;
const VARFLOAT_CODE: u8 = 5;
const VARBYTE_CODE: u8 = 6;
const STRING_CODE: u8 = 7;

pub fn send_batch(stream: &mut TcpStream,
                  inputs: &Vec<RpcPredictRequest>,
                  input_type: &InputType)
                  -> Vec<Output> {
    assert!(inputs.len() > 0);
    let message = match input_type {
        &InputType::Integer(l) => {
            if l < 0 {
                encode_var_ints(inputs)
            } else {
                encode_fixed_ints(inputs, l)
            }
        }
        &InputType::Float(l) => {
            if l < 0 {
                encode_var_floats(inputs)
            } else {
                encode_fixed_floats(inputs, l)
            }
        }
        &InputType::Byte(l) => {
            if l < 0 {
                encode_var_bytes(inputs)
            } else {
                encode_fixed_bytes(inputs, l)
            }
        }
        &InputType::Str => encode_strs(inputs),
    };
    stream.write_all(&message[..]).unwrap();
    stream.flush().unwrap();

    let num_response_bytes = inputs.len() * mem::size_of::<Output>();
    let mut response_buffer: Vec<u8> = vec![0; num_response_bytes];
    stream.read_exact(&mut response_buffer).unwrap();
    let mut cursor = Cursor::new(response_buffer);
    let mut responses: Vec<Output> = Vec::with_capacity(inputs.len());
    for i in 0..inputs.len() {
        responses.push(cursor.read_f64::<LittleEndian>().unwrap());
    }
    responses
}

pub fn shutdown(stream: &mut TcpStream) -> bool {
    let mut message = Vec::new();
    message.push(SHUTDOWN_CODE);
    message.write_u32::<LittleEndian>(0 as u32).unwrap();
    stream.write_all(&message[..]).unwrap();
    stream.flush().unwrap();
    let mut response_buffer: Vec<u8> = vec![0; 4];
    stream.read_exact(&mut response_buffer).unwrap();
    let mut cursor = Cursor::new(response_buffer);
    let response = cursor.read_u32::<LittleEndian>().unwrap();
    // info!("SHUTDOWN RESPONSE: {}", response);
    response == 1234_u32
}

pub fn encode_var_ints(inputs: &Vec<RpcPredictRequest>) -> Vec<u8> {
    // let mut message_len = 5; // 1 byte type + 4 bytes num inputs
    let mut message = Vec::new();
    message.push(VARINT_CODE);
    message.write_u32::<LittleEndian>(inputs.len() as u32).unwrap();
    let intsize = mem::size_of::<i32>();
    // number of bytes used to encode the content
    let mut content_len = 0;
    for x in inputs.iter() {
        match x.input {
            // for each input: 4 bytes length + len*sizeof(int)
            Input::Ints {i: ref f, length: _} => content_len += f.len() * intsize,
            _ => unreachable!(),
        }
    }
    content_len += mem::size_of::<u32>() * inputs.len();
    message.write_u32::<LittleEndian>(content_len as u32).unwrap();
    assert!(message.len() == 9);
    for x in inputs.iter() {
        match x.input {
            // for each input: 4 bytes length + len*sizeof(int)
            Input::Ints {i: ref f, length: _} => {
                message.write_u32::<LittleEndian>(f.len() as u32).unwrap();
                for xi in f.iter() {
                    message.write_i32::<LittleEndian>(*xi).unwrap();
                }
            }
            _ => unreachable!(),
        }
    }
    message
}

pub fn decode_var_ints(bytes: &mut Vec<u8>) -> Vec<Vec<i32>> {
    let mut cursor = Cursor::new(bytes);
    assert_eq!(VARINT_CODE, cursor.read_u8().unwrap());
    let num_inputs = cursor.read_u32::<LittleEndian>().unwrap();
    let content_len = cursor.read_u32::<LittleEndian>().unwrap();
    // let inp_len = cursor.read_u32::<LittleEndian>().unwrap();
    let mut responses = Vec::with_capacity(num_inputs as usize);
    for _ in 0..num_inputs {
        let inp_len = cursor.read_u32::<LittleEndian>().unwrap();
        let mut cur_response = Vec::with_capacity(inp_len as usize);
        for _ in 0..inp_len {
            cur_response.push(cursor.read_i32::<LittleEndian>().unwrap());
        }
        responses.push(cur_response);
    }
    responses
}

pub fn encode_fixed_ints(inputs: &Vec<RpcPredictRequest>, length: i32) -> Vec<u8> {
    // let mut message_len = 5; // 1 byte type + 4 bytes num inputs
    let mut message = Vec::new();
    message.push(FIXEDINT_CODE);
    message.write_u32::<LittleEndian>(inputs.len() as u32).unwrap();
    message.write_u32::<LittleEndian>(length as u32).unwrap();
    assert!(message.len() == 9);
    // let intsize = mem::size_of::<i32>;
    for x in inputs.iter() {
        match x.input {
            // for each input: 4 bytes length + len*sizeof(int)
            Input::Ints {i: ref f, length: _} => {
                for xi in f.iter() {
                    message.write_i32::<LittleEndian>(*xi).unwrap();
                }
            }
            _ => unreachable!(),
        }
    }
    message
}

pub fn decode_fixed_ints(bytes: &mut Vec<u8>) -> Vec<Vec<i32>> {
    let mut cursor = Cursor::new(bytes);
    assert_eq!(FIXEDINT_CODE, cursor.read_u8().unwrap());
    let num_inputs = cursor.read_u32::<LittleEndian>().unwrap();
    let inp_len = cursor.read_u32::<LittleEndian>().unwrap();
    let mut responses = Vec::with_capacity(num_inputs as usize);
    for _ in 0..num_inputs {
        let mut cur_response = Vec::with_capacity(inp_len as usize);
        for _ in 0..inp_len {
            cur_response.push(cursor.read_i32::<LittleEndian>().unwrap());
        }
        responses.push(cur_response);
    }
    responses
}

pub fn decode_fixed_bytes(bytes: &mut Vec<u8>) -> Vec<Vec<u8>> {
    let mut cursor = Cursor::new(bytes);
    assert_eq!(FIXEDBYTE_CODE, cursor.read_u8().unwrap());
    let num_inputs = cursor.read_u32::<LittleEndian>().unwrap();
    let inp_len = cursor.read_u32::<LittleEndian>().unwrap();
    let mut responses = Vec::with_capacity(num_inputs as usize);
    for _ in 0..num_inputs {
        let mut cur_response = Vec::with_capacity(inp_len as usize);
        for _ in 0..inp_len {
            cur_response.push(cursor.read_u8().unwrap());
        }
        responses.push(cur_response);
    }
    responses
}

pub fn decode_var_bytes(bytes: &mut Vec<u8>) -> Vec<Vec<u8>> {
    let mut cursor = Cursor::new(bytes);
    assert_eq!(VARBYTE_CODE, cursor.read_u8().unwrap());
    let num_inputs = cursor.read_u32::<LittleEndian>().unwrap();
    let content_len = cursor.read_u32::<LittleEndian>().unwrap();
    let mut responses = Vec::with_capacity(num_inputs as usize);
    for _ in 0..num_inputs {
        let inp_len = cursor.read_u32::<LittleEndian>().unwrap();
        let mut cur_response = Vec::with_capacity(inp_len as usize);
        for _ in 0..inp_len {
            cur_response.push(cursor.read_u8().unwrap());
        }
        responses.push(cur_response);
    }
    responses
}

pub fn decode_strs(bytes: &mut Vec<u8>) -> Vec<String> {
    let mut cursor = Cursor::new(bytes);
    assert_eq!(STRING_CODE, cursor.read_u8().unwrap());
    let num_inputs = cursor.read_u32::<LittleEndian>().unwrap();
    let content_len = cursor.read_u32::<LittleEndian>().unwrap();
    let mut str_lens = Vec::new();
    for _ in 0..num_inputs {
        str_lens.push(cursor.read_u32::<LittleEndian>().unwrap());
    }
    let mut decoder = Decoder::new(cursor).unwrap();
    let mut decompressed = Vec::new();
    decoder.read_to_end(&mut decompressed).unwrap();

    let mut outputs = Vec::new();
    for str_len in str_lens.iter() {
        let new_decomp = decompressed.split_off(*str_len as usize);
        let output = decompressed.clone();
        decompressed = new_decomp;
        let output_str = String::from_utf8(output).expect("Failed to parse string from bytes");
        outputs.push(output_str);
    }
    outputs
}

fn encode_fixed_floats(inputs: &Vec<RpcPredictRequest>, length: i32) -> Vec<u8> {
    let mut message = Vec::new();
    message.push(FIXEDFLOAT_CODE);
    message.write_u32::<LittleEndian>(inputs.len() as u32).unwrap();
    message.write_u32::<LittleEndian>(length as u32).unwrap();
    assert!(message.len() == 9);
    // let floatsize = mem::size_of::<f64>;
    for x in inputs.iter() {
        match x.input {
            Input::Floats {ref f, length: _} => {
                for xi in f.iter() {
                    message.write_f64::<LittleEndian>(*xi).unwrap();
                }
            }
            _ => unreachable!(),
        }
    }
    message
}

fn encode_var_floats(inputs: &Vec<RpcPredictRequest>) -> Vec<u8> {
    let mut message = Vec::new();
    message.push(VARFLOAT_CODE);
    message.write_u32::<LittleEndian>(inputs.len() as u32).unwrap();
    let floatsize = mem::size_of::<f64>();
    let mut content_len = 0;
    for x in inputs.iter() {
        match x.input {
            Input::Floats {ref f, length: _} => content_len += f.len() * floatsize,
            _ => unreachable!(),
        }
    }
    content_len += mem::size_of::<u32>() * inputs.len();
    message.write_u32::<LittleEndian>(content_len as u32).unwrap();
    assert!(message.len() == 9);
    for x in inputs.iter() {
        match x.input {
            Input::Floats {ref f, length: _} => {
                message.write_u32::<LittleEndian>(f.len() as u32).unwrap();
                for xi in f.iter() {
                    message.write_f64::<LittleEndian>(*xi).unwrap();
                }
            }
            _ => unreachable!(),
        }
    }
    message
}

pub fn encode_fixed_bytes(inputs: &Vec<RpcPredictRequest>, length: i32) -> Vec<u8> {
    let mut message = Vec::new();
    message.push(FIXEDBYTE_CODE);
    message.write_u32::<LittleEndian>(inputs.len() as u32).unwrap();
    message.write_u32::<LittleEndian>(length as u32).unwrap();
    assert!(message.len() == 9);
    for x in inputs.iter() {
        match x.input {
            Input::Bytes {ref b, length: _} => {
                for xi in b.iter() {
                    message.write_u8(*xi).unwrap();
                }
            }
            _ => unreachable!(),
        }
    }
    message
}

pub fn encode_var_bytes(inputs: &Vec<RpcPredictRequest>) -> Vec<u8> {
    let mut message = Vec::new();
    message.push(VARBYTE_CODE);
    message.write_u32::<LittleEndian>(inputs.len() as u32).unwrap();
    let mut content_len = 0;
    for x in inputs.iter() {
        match x.input {
            Input::Bytes {ref b, length: _} => content_len += b.len(),
            _ => unreachable!(),
        }
    }
    content_len += mem::size_of::<u32>() * inputs.len();
    message.write_u32::<LittleEndian>(content_len as u32).unwrap();
    assert!(message.len() == 9);
    for x in inputs.iter() {
        match x.input {
            Input::Bytes {ref b, length: _} => {
                message.write_u32::<LittleEndian>(b.len() as u32).unwrap();
                for xi in b.iter() {
                    message.write_u8(*xi).unwrap();
                }
            }
            _ => unreachable!(),
        }
    }
    message
}

pub fn encode_strs(inputs: &Vec<RpcPredictRequest>) -> Vec<u8> {
    let mut message = Vec::new();
    message.push(STRING_CODE);
    message.write_u32::<LittleEndian>(inputs.len() as u32).unwrap();
    // Compress the string content and write the compressed length to the output vector
    let mut compressor = EncoderBuilder::new().build(Vec::new()).unwrap();
    for x in inputs.iter() {
        match x.input {
            Input::Str {ref s} => compressor.write_all(s.as_bytes()).unwrap(),
            _ => unreachable!(),
        }
    }
    // Write a newline to prevent decompression from truncating the last character of 
    // the string
    compressor.write_all("\n".as_bytes()).unwrap();
    let (compressed_str, result) = compressor.finish();
    if let Err(e) = result {
        panic!("Failed to compress string input for RPC transmission. Error: {}", e.description());
    }
    let content_len = compressed_str.len() + (mem::size_of::<u32>() * inputs.len());
    message.write_u32::<LittleEndian>(content_len as u32).unwrap();
    // Write the length of each uncompressed string to the output vector
    for x in inputs.iter() {
        match x.input {
            Input::Str {ref s} => message.write_u32::<LittleEndian>(s.len() as u32).unwrap(),
            _ => unreachable!(),
        }
    }
    message.append(&mut compressed_str.to_owned());
    message
}

#[cfg(test)]
mod tests {
    use super::*;
    // use byteorder::{LittleEndian, WriteBytesExt, ReadBytesExt};
    // use std::io::Read;
    // use std::mem;
    use server::Input;
    use batching::RpcPredictRequest;
    use rand::{thread_rng, Rng};
    use time;

    fn random_ints(d: usize) -> Vec<i32> {
        let mut rng = thread_rng();
        rng.gen_iter::<i32>().take(d).collect::<Vec<i32>>()
    }

    fn random_bytes(d: usize) -> Vec<u8> {
        let mut rng = thread_rng();
        rng.gen_iter::<u8>().take(d).collect::<Vec<u8>>()
    }

    #[test]
    fn fixed_ints() {
        // let inp_length = 4;
        let mut rng = thread_rng();
        let inp_length = rng.gen_range::<usize>(0, 3000);
        let mut reqs: Vec<RpcPredictRequest> = Vec::new();
        let mut orig_inputs = Vec::new();
        for _ in 0..7 {
            let v = random_ints(inp_length);
            orig_inputs.push(v.clone());
            let r = RpcPredictRequest {
                input: Input::Ints {
                    i: v,
                    length: inp_length as i32,
                },
                recv_time: time::PreciseTime::now(),
            };
            reqs.push(r);
        }
        let mut encoded_ints = encode_fixed_ints(&reqs, inp_length as i32);
        let decoded_vecs = decode_fixed_ints(&mut encoded_ints);
        assert_eq!(decoded_vecs.len(), 7);
        for i in 0..decoded_vecs.len() {
            assert_eq!(&decoded_vecs[i][..], &orig_inputs[i][..]);
        }
    }

    #[test]
    fn var_ints() {
        let mut reqs: Vec<RpcPredictRequest> = Vec::new();
        let mut orig_inputs = Vec::new();
        let mut rng = thread_rng();
        for _ in 0..7 {
            let inp_length = rng.gen_range::<usize>(0, 3000);
            let v = random_ints(inp_length);
            orig_inputs.push(v.clone());
            let r = RpcPredictRequest {
                input: Input::Ints { i: v, length: -1 },
                recv_time: time::PreciseTime::now(),
            };
            reqs.push(r);
        }
        let mut encoded_ints = encode_var_ints(&reqs);
        let decoded_vecs = decode_var_ints(&mut encoded_ints);
        assert_eq!(decoded_vecs.len(), 7);
        for i in 0..decoded_vecs.len() {
            assert_eq!(&decoded_vecs[i][..], &orig_inputs[i][..]);
        }
    }

    #[test]
    fn fixed_bytes() {
        let mut rng = thread_rng();
        let inp_length = rng.gen_range::<usize>(0, 3000);
        let mut reqs: Vec<RpcPredictRequest> = Vec::new();
        let mut orig_inputs = Vec::new();
        for _ in 0..7 {
            let v = random_bytes(inp_length);
            orig_inputs.push(v.clone());
            let r = RpcPredictRequest {
                input: Input::Bytes {
                    b: v,
                    length: inp_length as i32,
                },
                recv_time: time::PreciseTime::now(),
            };
            reqs.push(r);
        }
        let mut encoded_bytes = encode_fixed_bytes(&reqs, inp_length as i32);
        let decoded_bytes = decode_fixed_bytes(&mut encoded_bytes);
        for i in 0..decoded_bytes.len() {
            assert_eq!(&decoded_bytes[i][..], &orig_inputs[i][..]);
        }
    }

    #[test]
    fn var_bytes() {
        let mut reqs: Vec<RpcPredictRequest> = Vec::new();
        let mut orig_inputs = Vec::new();
        let mut rng = thread_rng();
        for _ in 0..7 {
            let inp_length = rng.gen_range::<usize>(0, 3000);
            let v = random_bytes(inp_length);
            orig_inputs.push(v.clone());
            let r = RpcPredictRequest {
                input: Input::Bytes { b: v, length: -1 },
                recv_time: time::PreciseTime::now(),
            };
            reqs.push(r);
        }
        let mut encoded_bytes = encode_var_bytes(&reqs);
        let decoded_bytes = decode_var_bytes(&mut encoded_bytes);
        assert_eq!(decoded_bytes.len(), 7);
        for i in 0..decoded_bytes.len() {
            assert_eq!(&decoded_bytes[i][..], &orig_inputs[i][..]);
        }
    }

    #[test]
    fn strings() {
        let mut reqs: Vec<RpcPredictRequest> = Vec::new();
        let mut strs = Vec::new();
        strs.push("cats");
        strs.push("afsdiofjsdoifssd");
        strs.push("92842jcwf*0azxm$$__ ");
        strs.push("oceanic\n");
        strs.push("");
        for x in strs.iter() {
            let r = RpcPredictRequest {
                input: Input::Str { s: x.to_string() },
                recv_time: time::PreciseTime::now(),
            };
            reqs.push(r);
        }
        let mut encoded_strs = encode_strs(&reqs);
        let decoded_strs = decode_strs(&mut encoded_strs);
        assert_eq!(decoded_strs.len(), strs.len());
        for i in 0..decoded_strs.len() {
            assert_eq!(&decoded_strs[i][..], &strs[i][..]);
        }
    }

}
