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

    #[error("El perfil de esta instancia ya no existe. Edita la instancia y elige otro perfil.")]
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

    #[error("Ese tipo de instancia aun no esta disponible.")]
    LoaderNotSupported,

    #[error("{0} no esta disponible para esta version de Minecraft.")]
    LoaderUnavailable(String),

    #[error("La cuenta no esta disponible: {0}")]
    Identity(String),

    #[error("Tu sesion de Microsoft vencio. Vuelve a iniciar sesion en Perfiles.")]
    SignInRequired,

    #[error("Ya hay un inicio de sesion en curso.")]
    LoginInProgress,

    #[error("Este perfil lo usan {0} instancia(s). Cambia el perfil de esas instancias antes de quitarlo.")]
    ProfileInUse(usize),

    #[error("No se encontro el perfil.")]
    AccountNotFound,

    #[error("No se pudo leer o guardar la informacion del launcher.")]
    Storage,
}

/// Que accion puede ofrecer la interfaz ante un fallo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ErrorKind {
    /// Falta conexion: reintentar cuando haya internet.
    Offline,
    /// Archivos del juego danados o incompletos: reparar.
    Repairable,
    /// La sesion de la cuenta vencio: volver a iniciar sesion.
    SignIn,
    Other,
}

impl LauncherError {
    pub fn kind(&self) -> ErrorKind {
        match self {
            Self::Offline => ErrorKind::Offline,
            Self::SignInRequired => ErrorKind::SignIn,
            Self::CorruptDownload | Self::InstallFailed | Self::MissingGameFiles => ErrorKind::Repairable,
            _ => ErrorKind::Other,
        }
    }
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
                Self::Identity("el inicio de sesion con Microsoft no esta configurado en esta version.".into())
            }
            E::MicrosoftReauthRequired => Self::SignInRequired,
            E::MicrosoftAuthorizationCancelled => Self::Identity("se cancelo el inicio de sesion.".into()),
            E::MicrosoftLoginTimedOut => Self::Identity("el inicio de sesion tardo demasiado. Vuelve a intentarlo.".into()),
            E::MicrosoftStateMismatch => {
                Self::Identity("no se pudo verificar el inicio de sesion. Vuelve a intentarlo.".into())
            }
            E::MicrosoftXstsFailed(message) => Self::Identity(message),
            E::MicrosoftXboxAuthFailed(_) => Self::Identity("Xbox no acepto el inicio de sesion. Vuelve a intentarlo.".into()),
            E::MinecraftServicesRejectedApp => Self::Identity(
                "Minecraft no acepto esta aplicacion para iniciar sesion. Puedes seguir jugando con un perfil sin conexion; no se usara automaticamente.".into(),
            ),
            E::MinecraftOwnershipUnavailable => {
                Self::Identity("esta cuenta no tiene Minecraft Java Edition.".into())
            }
            E::SecureStoreUnavailable(detail) => {
                tracing::error!(%detail, "almacen seguro no disponible");
                Self::Identity("no se pudo acceder al almacen seguro del sistema.".into())
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

impl From<diagnostics::RepairError> for LauncherError {
    fn from(e: diagnostics::RepairError) -> Self {
        match e {
            diagnostics::RepairError::Install(inner) => inner.into(),
        }
    }
}

impl From<loaders::LoaderError> for LauncherError {
    fn from(e: loaders::LoaderError) -> Self {
        use loaders::LoaderError as E;
        tracing::error!(error = %e, "error de loader");
        match e {
            E::Http(h) if is_network(&h) => Self::Offline,
            E::Download(download_manager::DownloadError::Http(h)) if is_network(&h) => Self::Offline,
            E::DownloadFailures(_) => Self::Offline,
            E::NoVersionFor(_) => Self::LoaderUnavailable("Ese cargador de mods".into()),
            _ => Self::InstallFailed,
        }
    }
}
