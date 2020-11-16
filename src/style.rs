use anyhow::*;
use bevy::asset::{LoadContext, AssetLoader, LoadedAsset, AssetIoError};
use bevy::type_registry::TypeUuid;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use pixel_widgets::loader::Loader;
use std::collections::HashMap;
use bevy::render::renderer::TextureId;

#[derive(TypeUuid)]
#[uuid = "182aa3fa-a529-4096-a26b-9b49dc5577a3"]
pub struct Stylesheet {
    pub(crate) style: Arc<pixel_widgets::prelude::Style>,
    pub(crate) textures: HashMap<usize, TextureId>,
}

#[derive(Default)]
pub struct StylesheetLoader;

struct LoadContextLoader<'a>(&'a LoadContext<'a>);

impl<'a> Loader for LoadContextLoader<'a> {
    #[allow(clippy::type_complexity)]
    type Load = Pin<Box<dyn Future<Output = Result<Vec<u8>, Self::Error>> + Send + 'a>>;
    type Wait = Pin<Box<dyn Future<Output = Result<(), Self::Error>> + Send>>;
    type Error = AssetIoError;

    fn load(&self, url: impl AsRef<str>) -> Self::Load {
        Box::pin(self.0.read_asset_bytes(url.as_ref().to_string()))
    }

    fn wait(&self, _url: impl AsRef<str>) -> Self::Wait {
        unimplemented!()
    }
}

impl AssetLoader for StylesheetLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut LoadContext<'_>,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + 'a + Send>> {
        Box::pin(async move {
            let loader = LoadContextLoader(load_context);
            let style = pixel_widgets::prelude::Style::load_from_memory(bytes, &loader, 512, 0).await?;
            load_context.set_default_asset(LoadedAsset::new(Stylesheet {
                style: Arc::new(style),
                textures: Default::default(),
            }));
            Ok(())
        })
    }

    fn extensions(&self) -> &[&str] {
        &["pwss"]
    }
}
