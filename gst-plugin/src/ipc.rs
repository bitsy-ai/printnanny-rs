use std::collections::BTreeMap;

use polars::export::arrow::io::ipc;
use polars::io::json::{JsonFormat, JsonWriter};
use polars::prelude::*;

use crate::error::SerializationError;

// dataframe will be re-chunked in-place, then serialized to arrow streaming ipc message
// NOTE: streaming ipc and file-based ipcs use different memory layouts!
// The streaming IPC layout encapsulaes an unbounded sequence of messages:
// https://arrow.apache.org/docs/format/Columnar.html#ipc-streaming-format
// The stream writer can signal end-of-stream (EOS) with 8 bytes containing a 4-byte continuation indicator (0xFFFFFFFF) followed by 0 metadata length (0x00000000)
// The streaming protocol DOES NOT support random access!
// panics
pub fn dataframe_to_arrow_streaming_ipc_message(
    df: &mut DataFrame,
    metadata: Option<BTreeMap<String, String>>,
) -> Result<Vec<u8>, SerializationError> {
    let arrow_schema = match metadata {
        Some(m) => {
            let arrow_schema = df.schema().to_arrow();
            arrow_schema.with_metadata(m)
        }
        None => df.schema().to_arrow(),
    };

    // create a buffed memory writer
    let mut bufwriter = std::io::BufWriter::new(Vec::new());
    // initialize ipc stream writer
    let mut ipcwriter = ipc::write::StreamWriter::new(
        &mut bufwriter,
        ipc::write::WriteOptions { compression: None },
    );
    ipcwriter.start(&arrow_schema, None)?;
    df.rechunk();
    for batch in df.iter_chunks() {
        ipcwriter.write(&batch, None)?;
    }
    ipcwriter.finish()?;

    let arrow_msg = bufwriter
        .into_inner()
        .map_err(|_| SerializationError::BufferError)?;
    Ok(arrow_msg)
}

pub fn dataframe_to_json(df: &mut DataFrame) -> Result<Vec<u8>, SerializationError> {
    let mut bufwriter = std::io::BufWriter::new(Vec::new());
    let mut jsonwriter = JsonWriter::new(&mut bufwriter).with_json_format(JsonFormat::JsonLines);
    jsonwriter.finish(df)?;
    let output = bufwriter
        .into_inner()
        .map_err(|_| SerializationError::BufferError)?;
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dataframe_to_json() {
        let mut dataframe = df!(
            "x0" => vec![0; 10],
            "x1" => vec![1; 10]
        )
        .unwrap();

        let expected = [
            123, 34, 120, 48, 34, 58, 48, 44, 34, 120, 49, 34, 58, 49, 125, 10, 123, 34, 120, 48,
            34, 58, 48, 44, 34, 120, 49, 34, 58, 49, 125, 10, 123, 34, 120, 48, 34, 58, 48, 44, 34,
            120, 49, 34, 58, 49, 125, 10, 123, 34, 120, 48, 34, 58, 48, 44, 34, 120, 49, 34, 58,
            49, 125, 10, 123, 34, 120, 48, 34, 58, 48, 44, 34, 120, 49, 34, 58, 49, 125, 10, 123,
            34, 120, 48, 34, 58, 48, 44, 34, 120, 49, 34, 58, 49, 125, 10, 123, 34, 120, 48, 34,
            58, 48, 44, 34, 120, 49, 34, 58, 49, 125, 10, 123, 34, 120, 48, 34, 58, 48, 44, 34,
            120, 49, 34, 58, 49, 125, 10, 123, 34, 120, 48, 34, 58, 48, 44, 34, 120, 49, 34, 58,
            49, 125, 10, 123, 34, 120, 48, 34, 58, 48, 44, 34, 120, 49, 34, 58, 49, 125, 10,
        ];

        let b = dataframe_to_json(&mut dataframe).unwrap();
        assert_eq!(b, expected);
    }

    #[test]
    fn test_dataframe_to_arrow_streaming_ipc_message() {
        let mut dataframe = df!(
            "x0" => vec![0; 10],
            "x1" => vec![1; 10]
        )
        .unwrap();

        let expected = [
            255, 255, 255, 255, 192, 0, 0, 0, 4, 0, 0, 0, 242, 255, 255, 255, 20, 0, 0, 0, 4, 0, 1,
            0, 0, 0, 10, 0, 11, 0, 8, 0, 10, 0, 4, 0, 248, 255, 255, 255, 12, 0, 0, 0, 8, 0, 8, 0,
            0, 0, 4, 0, 2, 0, 0, 0, 76, 0, 0, 0, 4, 0, 0, 0, 236, 255, 255, 255, 56, 0, 0, 0, 32,
            0, 0, 0, 24, 0, 0, 0, 1, 2, 0, 0, 16, 0, 18, 0, 4, 0, 16, 0, 17, 0, 8, 0, 0, 0, 12, 0,
            0, 0, 0, 0, 244, 255, 255, 255, 32, 0, 0, 0, 1, 0, 0, 0, 8, 0, 9, 0, 4, 0, 8, 0, 2, 0,
            0, 0, 120, 49, 0, 0, 236, 255, 255, 255, 56, 0, 0, 0, 32, 0, 0, 0, 24, 0, 0, 0, 1, 2,
            0, 0, 16, 0, 18, 0, 4, 0, 16, 0, 17, 0, 8, 0, 0, 0, 12, 0, 0, 0, 0, 0, 244, 255, 255,
            255, 32, 0, 0, 0, 1, 0, 0, 0, 8, 0, 9, 0, 4, 0, 8, 0, 2, 0, 0, 0, 120, 48, 0, 0, 255,
            255, 255, 255, 184, 0, 0, 0, 4, 0, 0, 0, 236, 255, 255, 255, 128, 0, 0, 0, 0, 0, 0, 0,
            20, 0, 0, 0, 4, 0, 3, 0, 12, 0, 19, 0, 16, 0, 18, 0, 12, 0, 4, 0, 230, 255, 255, 255,
            10, 0, 0, 0, 0, 0, 0, 0, 96, 0, 0, 0, 20, 0, 0, 0, 0, 0, 0, 0, 0, 0, 10, 0, 20, 0, 4,
            0, 12, 0, 16, 0, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 40, 0, 0, 0, 0, 0, 0, 0, 64, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 64, 0, 0, 0, 0, 0, 0, 0, 40, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 10, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 10, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0,
            0, 1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 255, 255, 255, 255, 0, 0, 0, 0,
        ]
        .map(|v| v as u8);

        let b = dataframe_to_arrow_streaming_ipc_message(&mut dataframe, None).unwrap();
        assert_eq!(b, expected);
    }

    #[test]
    fn test_dataframe_to_arrow_streaming_ipc_message_with_metadata() {
        let mut dataframe = df!(
            "x0" => vec![0; 10],
            "x1" => vec![1; 10]
        )
        .unwrap();

        let expected = [
            255, 255, 255, 255, 56, 1, 0, 0, 4, 0, 0, 0, 242, 255, 255, 255, 20, 0, 0, 0, 4, 0, 1,
            0, 0, 0, 10, 0, 11, 0, 8, 0, 10, 0, 4, 0, 242, 255, 255, 255, 128, 0, 0, 0, 16, 0, 0,
            0, 0, 0, 10, 0, 12, 0, 0, 0, 4, 0, 8, 0, 2, 0, 0, 0, 56, 0, 0, 0, 4, 0, 0, 0, 244, 255,
            255, 255, 24, 0, 0, 0, 12, 0, 0, 0, 8, 0, 12, 0, 4, 0, 8, 0, 2, 0, 0, 0, 49, 53, 0, 0,
            12, 0, 0, 0, 102, 114, 97, 109, 101, 95, 114, 97, 116, 101, 95, 110, 0, 0, 0, 0, 244,
            255, 255, 255, 24, 0, 0, 0, 12, 0, 0, 0, 8, 0, 12, 0, 4, 0, 8, 0, 2, 0, 0, 0, 49, 53,
            0, 0, 12, 0, 0, 0, 102, 114, 97, 109, 101, 95, 114, 97, 116, 101, 95, 100, 0, 0, 0, 0,
            2, 0, 0, 0, 76, 0, 0, 0, 4, 0, 0, 0, 236, 255, 255, 255, 56, 0, 0, 0, 32, 0, 0, 0, 24,
            0, 0, 0, 1, 2, 0, 0, 16, 0, 18, 0, 4, 0, 16, 0, 17, 0, 8, 0, 0, 0, 12, 0, 0, 0, 0, 0,
            244, 255, 255, 255, 32, 0, 0, 0, 1, 0, 0, 0, 8, 0, 9, 0, 4, 0, 8, 0, 2, 0, 0, 0, 120,
            49, 0, 0, 236, 255, 255, 255, 56, 0, 0, 0, 32, 0, 0, 0, 24, 0, 0, 0, 1, 2, 0, 0, 16, 0,
            18, 0, 4, 0, 16, 0, 17, 0, 8, 0, 0, 0, 12, 0, 0, 0, 0, 0, 244, 255, 255, 255, 32, 0, 0,
            0, 1, 0, 0, 0, 8, 0, 9, 0, 4, 0, 8, 0, 2, 0, 0, 0, 120, 48, 0, 0, 0, 0, 0, 0, 255, 255,
            255, 255, 184, 0, 0, 0, 4, 0, 0, 0, 236, 255, 255, 255, 128, 0, 0, 0, 0, 0, 0, 0, 20,
            0, 0, 0, 4, 0, 3, 0, 12, 0, 19, 0, 16, 0, 18, 0, 12, 0, 4, 0, 230, 255, 255, 255, 10,
            0, 0, 0, 0, 0, 0, 0, 96, 0, 0, 0, 20, 0, 0, 0, 0, 0, 0, 0, 0, 0, 10, 0, 20, 0, 4, 0,
            12, 0, 16, 0, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 40, 0, 0, 0, 0, 0, 0, 0, 64, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            64, 0, 0, 0, 0, 0, 0, 0, 40, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 10, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 10, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0,
            1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 255, 255, 255, 255, 0, 0, 0, 0,
        ]
        .map(|v| v as u8);

        let metadata = BTreeMap::from([
            ("frame_rate_n".to_string(), "15".to_string()),
            ("frame_rate_d".to_string(), "15".to_string()),
        ]);

        let b = dataframe_to_arrow_streaming_ipc_message(&mut dataframe, Some(metadata)).unwrap();
        assert_eq!(b, expected);
    }
}
