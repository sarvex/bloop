use std::sync::Arc;

use axum::{
    extract::{Path, Query},
    Extension, Json,
};

use super::prelude::*;

#[derive(Debug, serde::Deserialize, Default)]
pub(super) struct Params {
    /// 1-indexed line number at which to start the snippet
    pub line_start: Option<isize>,

    /// 1-indexed line number at which to end the snippet
    pub line_end: Option<usize>,
}

#[derive(serde::Serialize)]
pub(super) struct FileResponse {
    contents: String,
}

impl super::ApiResponse for FileResponse {}

pub(super) async fn handle<'a>(
    Path(path): Path<String>,
    Query(params): Query<Params>,
    Extension(indexes): Extension<Arc<Indexes>>,
) -> Result<Json<super::Response<'a>>, Error> {
    // Strip leading slash, always present.
    let file_disk_path = &path[1..];

    let doc = indexes
        .file
        .file_body(file_disk_path)
        .await
        .map_err(Error::internal)?;

    Ok(json(FileResponse {
        contents: split_by_lines(&doc.content, &doc.line_end_indices, &params)?.to_string(),
    }))
}

fn split_by_lines<'a>(text: &'a str, indices: &[u32], params: &Params) -> Result<&'a str, Error> {
    let char_start = match params.line_start {
        Some(line_start) if line_start == 1 => 0,
        Some(line_start) if line_start > 1 => {
            (indices
                .get(line_start as usize - 2)
                .ok_or_else(|| Error::user("invalid line number"))?
                + 1) as usize
        }
        Some(_) => return Err(Error::user("line numbers are 1-indexed!")),
        _ => 0,
    };

    let line_end = params.line_end.unwrap_or(indices.len()) - 1;
    let char_end = *indices
        .get(line_end)
        .ok_or_else(|| Error::user("invalid line number"))? as usize;

    Ok(&text[char_start..=char_end])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_params() {
        let text = r#"aaaaaa
bbbbbb
cccccc
"#;

        let indices = text
            .match_indices('\n')
            .map(|(i, _)| i as u32)
            .collect::<Vec<_>>();

        println!("{indices:?}");

        assert_eq!(
            split_by_lines(
                text,
                &indices,
                &Params {
                    line_start: None,
                    line_end: None
                }
            )
            .unwrap_or_else(|_| panic!("bad")),
            text
        );

        assert_eq!(
            split_by_lines(
                text,
                &indices,
                &Params {
                    line_start: Some(1),
                    line_end: None
                }
            )
            .unwrap_or_else(|_| panic!("bad")),
            text
        );

        assert_eq!(
            split_by_lines(
                text,
                &indices,
                &Params {
                    line_start: Some(2),
                    line_end: None
                }
            )
            .unwrap_or_else(|_| panic!("bad")),
            &text[7..]
        );

        assert_eq!(
            split_by_lines(
                text,
                &indices,
                &Params {
                    line_start: Some(3),
                    line_end: Some(3),
                }
            )
            .unwrap_or_else(|_| panic!("bad")),
            &text[14..]
        );

        assert_eq!(
            split_by_lines(
                text,
                &indices,
                &Params {
                    line_start: Some(2),
                    line_end: Some(3),
                }
            )
            .unwrap_or_else(|_| panic!("bad")),
            &text[7..]
        );
    }
}
