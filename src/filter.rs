use crate::data::Domains;
use anyhow::{bail, Context, Result};
use url::form_urlencoded;
use url::Url;

pub fn filter<'a>(domains: &Domains, url: &'a mut Url) -> Result<&'a Url> {
    let domain = url.domain();
    if domain.is_none() {
        bail!("this is an invalid URL")
    }
    let domain = domain.unwrap();
    let rule = domains
        .get(domain)
        .context(format!("no rule for domain: <{}>", domain))?;

    if rule.rules.is_empty() {
        bail!("no rule for domain: <{}>", domain)
    }

    let rule = &rule.rules;

    // take a copy of the query string for later use
    let ori_query = url.query();
    if ori_query.is_none() {
        return Ok(url);
    }
    // copy the original query string
    let ori_query = url.query().unwrap().to_string();
    let ori_query = form_urlencoded::parse(ori_query.as_bytes());

    // clean the query string
    url.set_query(None);

    for (key, val) in ori_query {
        let mut has_same = false;
        for r in rule {
            if key == r.as_str() {
                has_same = true;
                break;
            }
        }
        if !has_same {
            url.query_pairs_mut().append_pair(&key, &val);
        }
    }

    Ok(url)
}

#[test]
fn test_filter() {
    let data =
        crate::data::Domains::load_from_file("./rules.toml").expect("fail to read rules.toml");
    let mut url = Url::parse(
        "https://twitter.com/CiloRanko/status/1478401918792011776?s=20&t=AVPOmNLtaozrA0Ccp6DyAw",
    )
    .unwrap();
    let url = filter(&data, &mut url).unwrap();
    assert_eq!(
        url.as_str(),
        "https://twitter.com/CiloRanko/status/1478401918792011776"
    );
}