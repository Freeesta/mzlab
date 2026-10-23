//! The subagents of the data assistant for the pages of the site: the request of a role for the data it may see, and the verdict on its answer
//! (`mzlab_llm::subcheck`). The prompts and the registry of steps come from the page (the prompts are fetched on demand, the registry gives the limits of
//! the parameters); no network here. The page sends the request, hands the body back to `check`, and gives the orchestrator only an accepted summary
//! (`Agent.reading`). Used by `mzlab/web/llm/llm-agente.js`.
use mzlab_core::error::Error;
use mzlab_core::json::{parse, Out, Value};
use mzlab_core::step::{registry_from_value, Step};
use mzlab_llm::assist::{roles, with_prompts, Prompts, Role};
use mzlab_llm::provider::Provider;
use mzlab_llm::subcheck::{check, request, Verdict};
use wasm_bindgen::prelude::*;

use crate::{fail, find};

/// `[["file","text"],...]` as the prompts of the roles.
pub(crate) fn prompts_of(json: &str) -> Result<Prompts, String> {
    let v = parse(json.as_bytes()).map_err(fail)?;
    let pairs: Vec<(&str, &str)> = v
        .arr()
        .iter()
        .filter_map(|p| Some((p.arr().first()?.str()?, p.arr().get(1)?.str()?)))
        .collect();
    Ok(with_prompts(&pairs))
}

fn registry_of(json: &str) -> Result<Vec<Step>, String> {
    registry_from_value(&parse(json.as_bytes()).map_err(fail)?).map_err(fail)
}

fn call_error(field: &str) -> String {
    fail(Error::new("err.llm.assist.call").with("field", field))
}

/// A verdict as the JSON the page reads.
fn verdict_json(v: Verdict) -> String {
    let mut o = Out::default();
    match v {
        Verdict::Accepted(s) => o.raw("{\"verdict\":\"accepted\",\"summary\":").raw(&s),
        Verdict::Retry { request, problems } => o
            .raw("{\"verdict\":\"retry\",\"request\":")
            .raw(&request.to_json())
            .raw(",\"problems\":")
            .raw(&problems),
        Verdict::Dropped { problems } => o
            .raw("{\"verdict\":\"dropped\",\"problems\":")
            .raw(&problems),
    };
    o.raw("}");
    o.0
}

/// The roles, their prompts and the registry of steps, for one conversation.
#[wasm_bindgen]
pub struct Assist {
    provider: Provider,
    roles: Vec<Role>,
    prompts: Prompts,
    registry: Vec<Step>,
}

#[wasm_bindgen]
impl Assist {
    /// `roles_json`: `assist-ruoli/1`; `prompts_json`: `[["02_router.md","..."],...]`; `steps_json`: the `step/1` descriptors (the page has them).
    #[wasm_bindgen(constructor)]
    pub fn new(
        provider_id: &str,
        roles_json: &str,
        prompts_json: &str,
        steps_json: &str,
    ) -> Result<Assist, String> {
        let roles = roles(roles_json).map_err(fail)?;
        let prompts = prompts_of(prompts_json)?;
        prompts.check(&roles).map_err(fail)?;
        Ok(Assist {
            provider: find(provider_id)?,
            roles,
            prompts,
            registry: registry_of(steps_json)?,
        })
    }

    fn role(&self, name: &str) -> Result<(&Role, &str), String> {
        let r = self
            .roles
            .iter()
            .find(|r| r.role == name)
            .ok_or_else(|| call_error("role"))?;
        Ok((r, self.prompts.prompt(&r.prompt_file).unwrap_or("")))
    }

    fn input(json: &str) -> Result<Value, String> {
        parse(json.as_bytes()).map_err(fail)
    }

    /// The request (`{url, headers, body, auth}`, never the key) of `role` for `input_json`, the data blocks keyed by source.
    pub fn request(&self, role: &str, input_json: &str) -> Result<String, String> {
        let (r, prompt) = self.role(role)?;
        let req = request(&self.provider, r, prompt, &Self::input(input_json)?).map_err(fail)?;
        Ok(req.to_json())
    }

    /// The readers a router answer (`body`) asks for, as a JSON array of roles; an answer outside the schema asks for none (the router is advisory).
    pub fn readers(body: &str) -> String {
        let mut o = Out::default();
        o.raw("[");
        for (i, r) in mzlab_llm::router::parse(body)
            .map(|r| r.readers())
            .unwrap_or_default()
            .iter()
            .enumerate()
        {
            if i > 0 {
                o.raw(",");
            }
            o.str(r);
        }
        o.raw("]");
        o.0
    }

    /// SHA-256 of the prompt of `role`, for the log.
    pub fn digest(&self, role: &str) -> Result<String, String> {
        let (r, _) = self.role(role)?;
        self.prompts
            .digest(&r.prompt_file)
            .ok_or_else(|| call_error("prompt"))
    }

    /// The verdict on `body`, the answer number `attempt` (0 or 1) of `role` over `input_json`: `{"verdict":"accepted","summary":...}`,
    /// `{"verdict":"retry","request":...,"problems":[...]}` (send it, then call again with attempt 1) or `{"verdict":"dropped","problems":[...]}`.
    pub fn check(
        &self,
        role: &str,
        input_json: &str,
        body: &str,
        attempt: u8,
    ) -> Result<String, String> {
        let (r, prompt) = self.role(role)?;
        let input = Self::input(input_json)?;
        let v = check(
            &self.provider,
            r,
            prompt,
            &input,
            body,
            attempt,
            &self.registry,
        )
        .map_err(fail)?;
        Ok(verdict_json(v))
    }
}
