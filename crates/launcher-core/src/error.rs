/// Errores de la capa de aplicacion. Los mensajes (`Display`) estan pensados para
/// mostrarse tal cual a una persona no tecnica; el detalle va a los logs.
#[derive(Debug, thiserror::Error)]
pub enum LauncherError {
    #[error("{0}")]
    InvalidInput(String),

    #[error("Ya existe un perfil con ese nombre.")]
    DuplicateProfile,

    #[error("Primero crea un perfil para poder jugar.")]
    NoDefaultProfile,

    #[error("No se encontro el perfil asociado a esta instancia.")]
    ProfileMissing,

    #[error("No se encontro la instancia.")]
    InstanceNotFound,

    #[error("Esta instancia ya se esta ejecutando o preparando.")]
    AlreadyRunning,

    #[error("La instancia todavia se esta preparando y no se puede cerrar aun.")]
    NotYetRunning,

    #[error("Esta version de Minecraft no esta disponible.")]
    VersionUnavailable,

    #[error("Necesitas conexion a internet para descargar los archivos que faltan.")]
    Offline,

    #[error("Se descargo un archivo danado. Intentalo de nuevo.")]
    CorruptDownload,

    #[error("No se pudo preparar la instalacion de Minecraft.")]
    InstallFailed,

    #[error("No se encontro una version de Java compatible y no se pudo descargar.")]
    JavaUnavailable,

    #[error("Faltan archivos del juego. Vuelve a instalar o repara la instancia.")]
    MissingGameFiles,

    #[error("No se pudo iniciar Minecraft.")]
    LaunchFailed,

    #[error("La cuenta no esta disponible: {0}")]
    Identity(String),

    #[error("No se pudo leer o guardar la informacion del launcher.")]
    Storage,
}

impl From<account_storage::StorageError> for LauncherError {
    fn from(e: account_storage::StorageError) -> Self {
        tracing::error!(error = %e, "error de almacenamiento de cuentas");
        Self::Storage
    }
}

impl From<instance_manager::InstanceError> for LauncherError {
    fn from(e: instance_manager::InstanceError) -> Self {
        use instance_manager::InstanceError as E;
        match e {
            E::NotFound(_) | E::InvalidId => Self::InstanceNotFound,
            E::Invalid(msg) => Self::InvalidInput(msg),
            other => {
                tracing::error!(error = %other, "error de almacenamiento de instancias");
                Self::Storage
            }
        }
    }
}

impl From<identity_api::IdentityError> for LauncherError {
    fn from(e: identity_api::IdentityError) -> Self {
        use identity_api::IdentityError as E;
        match e {
            E::LocalProfileInvalid(msg) => Self::InvalidInput(msg),
            E::NetworkUnavailable => Self::Offline,
            E::MicrosoftNotConfigured => {
                Self::Identity("el inicio de sesion con Microsoft aun no esta disponible.".into())
            }
            other => {
                tracing::error!(error = %other, "error de identidad");
                Self::Identity("no se pudo comprobar la cuenta.".into())
            }
        }
    }
}

impl From<minecraft_launch::LaunchError> for LauncherError {
    fn from(e: minecraft_launch::LaunchError) -> Self {
        use minecraft_launch::LaunchError as E;
        tracing::error!(error = %e, "error de lanzamiento");
        match e {
            E::JavaMissing(_) | E::MissingFiles(_) => Self::MissingGameFiles,
            _ => Self::LaunchFailed,
        }
    }
}

pub(crate) fn is_network(e: &reqwest::Error) -> bool {
    e.is_connect() || e.is_timeout() || e.is_request() || e.status().is_some()
}

impl From<minecraft_meta::MetaError> for LauncherError {
    fn from(e: minecraft_meta::MetaError) -> Self {
        tracing::error!(error = %e, "error de metadatos de Minecraft");
        match e {
            minecraft_meta::MetaError::Http(h) if is_network(&h) => Self::Offline,
            minecraft_meta::MetaError::VersionNotFound(_) => Self::VersionUnavailable,
            _ => Self::InstallFailed,
        }
    }
}

impl From<minecraft_install::InstallError> for LauncherError {
    fn from(e: minecraft_install::InstallError) -> Self {
        use minecraft_install::InstallError as E;
        tracing::error!(error = %e, "error de instalacion");
        match e {
            E::Download(download_manager::DownloadError::Http(h)) if is_network(&h) => Self::Offline,
            E::Download(download_manager::DownloadError::HashMismatch { .. }) => Self::CorruptDownload,
            E::Meta(m) => m.into(),
            E::DownloadFailures(_) => Self::Offline,
            _ => Self::InstallFailed,
        }
    }
}

impl From<java_runtime::JavaError> for LauncherError {
    fn from(e: java_runtime::JavaError) -> Self {
        tracing::error!(error = %e, "error de Java");
        match e {
            java_runtime::JavaError::Http(h) if is_network(&h) => Self::Offline,
            _ => Self::JavaUnavailable,
        }
    }
}
