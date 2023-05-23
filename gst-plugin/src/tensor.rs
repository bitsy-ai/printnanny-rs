use std::num::ParseIntError;

use arrow2::datatypes;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum TensorError {
    #[error("Failed to parse tensor types: {tensor_types}")]
    InvalidType { tensor_types: String },
    #[error("Failed to parse tensor shapes: {tensor_shapes}")]
    InvalidShape { tensor_shapes: String },
    #[error("Expected tensor_shapes={tensor_shapes}, tensor_types={tensor_types}, tensor_names={tensor_names} to be the same length.")]
    TensorLength {
        tensor_shapes: String,
        tensor_types: String,
        tensor_names: String,
    },
    #[error("Failed to parse integer from tensor shape slice")]
    ParseIntError {
        #[from]
        source: ParseIntError,
    },
}

// Parse a single tensor shape, with dimensions separated by ":" symbol, for example "40:1:1:4" -> [40,1,1,4]
pub fn parse_tensor_shape(tensor_shape: &str) -> Result<Vec<u32>, ParseIntError> {
    tensor_shape.split(':').map(|s| s.parse::<u32>()).collect()
}

// Parse a single tensor type from string, returning Arrow data type
pub fn parse_tensor_type(tensor_type: &str) -> datatypes::DataType {
    match tensor_type {
        "boolean" => datatypes::DataType::Boolean,
        "float32" => datatypes::DataType::Float32,
        "float64" => datatypes::DataType::Float64,
        "int32" => datatypes::DataType::Int32,
        "int64" => datatypes::DataType::Int64,
        _ => unimplemented!("parse_tensor_type is not implemented for {}", tensor_type),
    }
}

// Parse a comma-separated String of tensor types
pub fn parse_tensor_types(tensor_types: &str) -> Result<Vec<datatypes::DataType>, TensorError> {
    let parsed: Vec<&str> = tensor_types.split(',').collect();
    if !parsed.is_empty() {
        Ok(parsed.iter().map(|t| parse_tensor_type(t)).collect())
    } else {
        Err(TensorError::InvalidType {
            tensor_types: tensor_types.to_string(),
        })
    }
}

// Parse a comma-separated String of tensor shapes
pub fn parse_tensor_shapes(tensor_shapes: &str) -> Result<(usize, Vec<Vec<u32>>), TensorError> {
    // split individual tensor shapes
    let shape_per_tensor: Vec<&str> = tensor_shapes.split(',').collect();
    if !shape_per_tensor.is_empty() {
        let result: Result<Vec<Vec<u32>>, TensorError> = shape_per_tensor
            .iter()
            .map(|t| Ok(parse_tensor_shape(t)?))
            .collect();
        Ok((shape_per_tensor.len(), result?))
    } else {
        Err(TensorError::InvalidShape {
            tensor_shapes: tensor_shapes.to_string(),
        })
    }
}

// Parse comma-separated String of tensor names
pub fn parse_tensor_names(tensor_names: &str) -> Vec<String> {
    tensor_names.split(',').map(|s| s.to_string()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_parse_tensor_shapes() {
        let shapes = "4:40:1:1,40:1:1:1,40:1:1:1,1:1:1:1";

        let result = parse_tensor_shapes(shapes).unwrap();
        assert_eq!(
            result,
            (
                4,
                vec![
                    vec![4, 40, 1, 1],
                    vec![40, 1, 1, 1],
                    vec![40, 1, 1, 1],
                    vec![1, 1, 1, 1]
                ]
            )
        )
    }
    #[test]
    fn test_parse_tensor_types() {
        let types = "float32,float64,int32,int64";

        let result = parse_tensor_types(types).unwrap();
        assert_eq!(
            result,
            vec![
                datatypes::DataType::Float32,
                datatypes::DataType::Float64,
                datatypes::DataType::Int32,
                datatypes::DataType::Int64
            ]
        )
    }
}
