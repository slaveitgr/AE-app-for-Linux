pub fn html_to_easymark(html: &str) -> String {
    let mut rtn = html
        .to_string()
        .replace("\n", "")
        .replace("*", "\\*")
        .replace("<br>", "\n")
        .replace("<b>", "*")
        .replace("</b>", "*")
        .replace("<span style=\"font-size: small;\">", "") // WE DO NOT CARE 🗣️
        .replace("</span>", "")
        .replace("&nbsp;", " ")
        .replace("\\", "");
    let mut idx: usize = 0;

    // link corrector
    while rtn.len() > idx
        && rtn[idx..].find("<a href=\"").is_some()
        && rtn[idx..].find("\">").is_some()
    {
        let segment = rtn[idx..].to_string();

        let link_beg = segment.find("<a href=\"").unwrap() + 9;
        let link_end = segment[link_beg..].find("\"").unwrap() + link_beg;
        let text_beg = segment[link_end..].find("\">").unwrap() + link_end + 2;
        let text_end = segment[text_beg..].find("</a>").unwrap() + text_beg;

        let link_mod: String = "[".to_string()
            + &segment[text_beg..text_end].to_string()
            + "]("
            + &segment[link_beg..link_end].to_string()
            + ")";
        rtn = rtn[..idx + (link_beg - 9)].to_string()
            + &link_mod
            + &rtn[idx + text_end + 4..].to_string();

        idx += (link_beg - 9) + (link_mod.len());
    }
    idx = 0;

    // image removal, easymark doesn't support them and EAD doesn't seem to display them
    while rtn.len() > idx && rtn[idx..].find("<img").is_some() && rtn[idx..].find(">").is_some() {
        let segment = rtn[idx..].to_string();

        let img_beg = segment.find("<img").unwrap() + 4;
        let img_end = segment[img_beg..].find(">").unwrap() + img_beg;

        rtn = rtn[..idx + (img_beg - 4)].to_string() + &rtn[idx + img_end + 1..].to_string();

        idx += img_beg - 4;
    }

    rtn.replace("/", "\\/") // links are caught in the crossfire of this, but clicking them still leads to where they should
        .replace("<i>", "/")
        .replace("<\\/i>", "/")
}
