use crate::generator::CurrentPath;
use base64::{Engine, prelude::BASE64_STANDARD};
use comrak::{
    create_formatter,
    html::{ChildRendering, dangerous_url},
    nodes::NodeValue,
};
use percent_encoding::percent_decode_str;
use std::{fmt::Write, io::Read, path::PathBuf};

pub fn render_to_base64<'a>(paths: &'a CurrentPath<'a>, url: &str) -> Option<String> {
    let percent_decode = PathBuf::from(percent_decode_str(url).decode_utf8().ok()?.into_owned());
    let joined_path = if percent_decode.is_absolute() {
        paths
            .project_path
            .join(percent_decode.strip_prefix("/").ok()?)
    } else {
        paths.file_path.join(percent_decode)
    };

    let mut p = std::fs::File::open(&joined_path).ok()?;

    let mut vec = Vec::new();
    p.read_to_end(&mut vec).ok()?;

    let mimetype = infer::get(&vec)?;

    if !matches!(mimetype.matcher_type(), infer::MatcherType::Image) {
        return None;
    }

    Some(format!(
        "{};base64,{}",
        mimetype.mime_type(),
        BASE64_STANDARD.encode(&vec)
    ))
}

create_formatter!(CustomMath<&'a CurrentPath<'a>>, {
    NodeValue::Math(ref node) => |context, entering| {
        let fence = if node.display_math {
            "$$"
        } else {
            "$"
        };

        context.write_str(fence)?;
        if entering {
            context.write_str(&node.literal)?;
        }
    },
    NodeValue::Image(ref nl) => |context, node, entering| {
        if entering {
            if context.options.render.figure_with_caption {
                context.write_str("<figure>")?;
            }
            context.write_str("<img")?;
            if context.options.render.sourcepos {
                let ast = node.data();
                if ast.sourcepos.start.line > 0 {
                    write!(context, " data-sourcepos=\"{}\"", ast.sourcepos)?;
                }
            }
            context.write_str(" src=\"")?;
            let url = &nl.url;
            if context.options.render.r#unsafe || !dangerous_url(url) {
                if let Some(base64) = render_to_base64(context.user, url) {
                    context.write_str(&base64)?;
                } else if let Some(rewriter) = &context.options.extension.image_url_rewriter {
                    context.escape_href(&rewriter.to_html(&nl.url))?;
                } else {
                    context.escape_href(url)?;
                }
            }
            context.write_str("\" alt=\"")?;
            return Ok(ChildRendering::Plain);
        }
        if !nl.title.is_empty() {
            context.write_str("\" title=\"")?;
            context.escape(&nl.title)?;
        }
        context.write_str("\" />")?;
        if context.options.render.figure_with_caption {
            if !nl.title.is_empty() {
                context.write_str("<figcaption>")?;
                context.escape(&nl.title)?;
                context.write_str("</figcaption>")?;
            }
            context.write_str("</figure>")?;
        }

        return Ok(ChildRendering::HTML);
    },
});
