use std::collections::BTreeMap;

use ail_compiler::{
    CapabilityProvider, OutboundBatchCheck, OutboundCapabilityRequest, OutboundProviderOutcome,
    OutboundRequestHandle, RuntimeFault, RuntimeValue, Span,
};
use ail_service_host::{ServiceHost, canonical_config, canonical_workspace};

#[derive(Default)]
struct NotFoundProvider {
    next: usize,
    ready: BTreeMap<OutboundRequestHandle, OutboundProviderOutcome>,
}
impl CapabilityProvider for NotFoundProvider {
    fn supports(&self, receiver: &str, interface: &str) -> bool {
        receiver == "dependency" && interface == "DependencyClient"
    }
    fn call(
        &mut self,
        _: &str,
        _: &str,
        _: &str,
        _: &[RuntimeValue],
    ) -> Result<RuntimeValue, RuntimeFault> {
        Err(RuntimeFault::new(
            "M32.UNSUPPORTED",
            Span::empty(0),
            std::iter::empty::<(&str, &str)>(),
            std::iter::empty::<(&str, &str)>(),
        ))
    }
    fn supports_outbound_batch(&self, _: &str, _: &str, _: &str) -> bool {
        true
    }
    fn start_outbound(
        &mut self,
        _: &OutboundCapabilityRequest,
    ) -> Result<OutboundRequestHandle, RuntimeFault> {
        let h = OutboundRequestHandle(format!("request-{}", self.next));
        self.next += 1;
        self.ready.insert(
            h.clone(),
            OutboundProviderOutcome::Returned(RuntimeValue::variant(
                "batch_lookup.types.LookupOutcome",
                "NotFound",
                None,
            )),
        );
        Ok(h)
    }
    fn check_outbound(
        &mut self,
        handles: &[OutboundRequestHandle],
    ) -> Result<OutboundBatchCheck, RuntimeFault> {
        Ok(OutboundBatchCheck {
            completed: handles.first().cloned().into_iter().collect(),
            cancelled: false,
        })
    }
    fn collect_outbound(
        &mut self,
        handle: &OutboundRequestHandle,
    ) -> Result<OutboundProviderOutcome, RuntimeFault> {
        self.ready.remove(handle).ok_or_else(|| {
            RuntimeFault::new(
                "M32.MISSING_HANDLE",
                Span::empty(0),
                std::iter::empty::<(&str, &str)>(),
                std::iter::empty::<(&str, &str)>(),
            )
        })
    }
}

#[tokio::main]
async fn main() {
    let workspace = canonical_workspace().expect("canonical M32 source must compile");
    let config = canonical_config(&workspace).expect("r1 metadata must exist");
    let host = ServiceHost::new(&workspace, Box::<NotFoundProvider>::default(), config)
        .expect("M32 pins must match compiler metadata");
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .expect("bind service host");
    axum::serve(listener, host.router())
        .await
        .expect("serve service host");
}
