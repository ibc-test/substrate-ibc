use crate::{context::Context};
use crate::Config;
use sp_std::borrow::Borrow;
use sp_std::collections::btree_map::BTreeMap;
use sp_std::fmt::{self, Debug};
use sp_std::sync::Arc;
use crate::prelude::{String, format};
use sp_std::borrow::ToOwned;
use sp_std::vec;
use ibc::core::ics26_routing::context::{Ics26Context, Module, ModuleId, RouterBuilder};

#[derive(Default)]
pub struct SubstrateRouterBuilder(Router);

impl RouterBuilder for SubstrateRouterBuilder {
	type Router = Router;

	fn add_route(mut self, module_id: ModuleId, module: impl Module) -> Result<Self, String> {
		match self.0 .0.insert(module_id, Arc::new(module)) {
			None => Ok(self),
			Some(_) => Err("Duplicate module_id".to_owned()),
		}
	}

	fn build(self) -> Self::Router {
		self.0
	}
}

#[derive(Default, Clone)]
pub struct Router(BTreeMap<ModuleId, Arc<dyn Module>>);

impl Debug for Router {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let mut keys = vec![];
		for (key, _) in self.0.iter() {
			keys.push(format!("{}", key));
		}

		write!(f, "Router(BTreeMap(key({:?})", keys.join(","))
	}
}

impl ibc::core::ics26_routing::context::Router for Router {
	fn get_route_mut(&mut self, module_id: &impl Borrow<ModuleId>) -> Option<&mut dyn Module> {
		self.0.get_mut(module_id.borrow()).and_then(Arc::get_mut)
	}

	fn has_route(&self, module_id: &impl Borrow<ModuleId>) -> bool {
		self.0.get(module_id.borrow()).is_some()
	}
}

impl<T: Config> Ics26Context for Context<T> {
	type Router = Router;

	fn router(&self) -> &Self::Router {
		&self.router
	}

	fn router_mut(&mut self) -> &mut Self::Router {
		&mut self.router
	}
}
