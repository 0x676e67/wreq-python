use pyo3::prelude::*;

define_enum!(
    /// Selects which client profile the request should look like.
    ///
    /// This controls the built-in TLS, HTTP/2, and header presets used for the
    /// request. Variants cover browser-style profiles as well as other clients,
    /// such as OkHttp.
    const, struct Emulation,
    Profile,
    wreq_util::Profile,
    Chrome100,
    Chrome101,
    Chrome104,
    Chrome105,
    Chrome106,
    Chrome107,
    Chrome108,
    Chrome109,
    Chrome110,
    Chrome114,
    Chrome116,
    Chrome117,
    Chrome118,
    Chrome119,
    Chrome120,
    Chrome123,
    Chrome124,
    Chrome126,
    Chrome127,
    Chrome128,
    Chrome129,
    Chrome130,
    Chrome131,
    Chrome132,
    Chrome133,
    Chrome134,
    Chrome135,
    Chrome136,
    Chrome137,
    Chrome138,
    Chrome139,
    Chrome140,
    Chrome141,
    Chrome142,
    Chrome143,
    Chrome144,
    Chrome145,
    Chrome146,
    Chrome147,
    Chrome148,
    Chrome149,
    Chrome150,
    Chrome151,
    Chrome152,

    Edge101,
    Edge122,
    Edge127,
    Edge131,
    Edge134,
    Edge135,
    Edge136,
    Edge137,
    Edge138,
    Edge139,
    Edge140,
    Edge141,
    Edge142,
    Edge143,
    Edge144,
    Edge145,
    Edge146,
    Edge147,
    Edge148,

    Firefox109,
    Firefox117,
    Firefox128,
    Firefox133,
    Firefox135,
    FirefoxPrivate135,
    FirefoxAndroid135,
    Firefox136,
    FirefoxPrivate136,
    Firefox139,
    Firefox142,
    Firefox143,
    Firefox144,
    Firefox145,
    Firefox146,
    Firefox147,
    Firefox148,
    Firefox149,
    Firefox150,
    Firefox151,

    SafariIos17_2,
    SafariIos17_4_1,
    SafariIos16_5,
    Safari15_3,
    Safari15_5,
    Safari15_6_1,
    Safari16,
    Safari16_5,
    Safari17_0,
    Safari17_2_1,
    Safari17_4_1,
    Safari17_5,
    Safari17_6,
    Safari18,
    SafariIPad18,
    Safari18_2,
    Safari18_3,
    Safari18_3_1,
    SafariIos18_1_1,
    Safari18_5,
    Safari26,
    Safari26_1,
    Safari26_2,
    Safari26_3,
    Safari26_4,
    SafariIos26,
    SafariIos26_2,
    SafariIPad26,
    SafariIpad26_2,

    OkHttp3_9,
    OkHttp3_11,
    OkHttp3_13,
    OkHttp3_14,
    OkHttp4_9,
    OkHttp4_10,
    OkHttp4_12,
    OkHttp5,

    Opera116,
    Opera117,
    Opera118,
    Opera119,
    Opera120,
    Opera121,
    Opera122,
    Opera123,
    Opera124,
    Opera125,
    Opera126,
    Opera127,
    Opera128,
    Opera129,
    Opera130,
    Opera131,
);

define_enum!(
    /// Selects which platform the client should look like.
    ///
    /// This mainly affects platform-specific headers and user-agent details.
    /// In most cases you can keep the default unless you need to match a
    /// specific Windows, macOS, Linux, Android, or iOS profile.
    const,
    Platform,
    wreq_util::Platform,
    Windows,
    MacOS,
    Linux,
    Android,
    IOS,
);

/// Represents the configuration options for emulating a client profile and platform.
///
/// The `Emulation` struct allows you to configure various aspects of profile and platform
/// emulation, including the profile, platform, and whether to enable certain features
/// like HTTP/2 or headers.
#[derive(Clone)]
#[pyclass(subclass, from_py_object)]
pub struct Emulation(pub wreq_util::Emulation);

#[pymethods]
impl Emulation {
    /// Create a new Emulation option instance.
    #[new]
    #[pyo3(signature = (
        profile = Profile::Chrome100,
        platform = Platform::MacOS,
        http2 = true,
        headers = true
    ))]
    fn new(profile: Profile, platform: Platform, http2: bool, headers: bool) -> Self {
        let emulation = wreq_util::Emulation::builder()
            .profile(profile.into_ffi())
            .platform(platform.into_ffi())
            .http2(http2)
            .headers(headers)
            .build();
        Self(emulation)
    }

    /// Creates a new random Emulation option instance.
    #[staticmethod]
    fn random() -> Self {
        Self(wreq_util::Emulation::random())
    }
}

/// A helper enum to allow accepting either a Profile or an Emulation in the same parameter.
#[derive(FromPyObject)]
pub enum EmulationLike {
    Profile(Profile),
    Emulation(Emulation),
}

impl wreq::IntoEmulation for EmulationLike {
    fn into_emulation(self) -> wreq::Emulation {
        match self {
            EmulationLike::Profile(profile) => profile.into_ffi().into_emulation(),
            EmulationLike::Emulation(inner) => inner.0.into_emulation(),
        }
    }
}
