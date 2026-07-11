"""
This module provides functionality for emulating various browsers and HTTP clients
to bypass detection and fingerprinting. It supports emulating Chrome, Firefox, Edge,
Safari, Opera, and OkHttp clients across different operating systems and versions.

The emulation system modifies HTTP/2 settings, TLS fingerprints, and request headers
to match the behavior of real browsers and clients, making requests appear more
authentic and less likely to be blocked by anti-bot systems.
"""

from enum import Enum, auto
from typing import ClassVar, final

__all__ = ["Emulation", "Profile", "Platform"]


@final
class Profile(Enum):
    r"""
    Selects which client profile the request should look like.

    This controls the built-in TLS, HTTP/2, and header presets used for the
    request. Variants cover browser-style profiles as well as other clients,
    such as OkHttp.
    """

    # Chrome versions
    Chrome100 = auto()
    Chrome101 = auto()
    Chrome104 = auto()
    Chrome105 = auto()
    Chrome106 = auto()
    Chrome107 = auto()
    Chrome108 = auto()
    Chrome109 = auto()
    Chrome110 = auto()
    Chrome114 = auto()
    Chrome116 = auto()
    Chrome117 = auto()
    Chrome118 = auto()
    Chrome119 = auto()
    Chrome120 = auto()
    Chrome123 = auto()
    Chrome124 = auto()
    Chrome126 = auto()
    Chrome127 = auto()
    Chrome128 = auto()
    Chrome129 = auto()
    Chrome130 = auto()
    Chrome131 = auto()
    Chrome132 = auto()
    Chrome133 = auto()
    Chrome134 = auto()
    Chrome135 = auto()
    Chrome136 = auto()
    Chrome137 = auto()
    Chrome138 = auto()
    Chrome139 = auto()
    Chrome140 = auto()
    Chrome141 = auto()
    Chrome142 = auto()
    Chrome143 = auto()
    Chrome144 = auto()
    Chrome145 = auto()
    Chrome146 = auto()
    Chrome147 = auto()
    Chrome148 = auto()
    Chrome149 = auto()

    # Microsoft Edge versions
    Edge101 = auto()
    Edge122 = auto()
    Edge127 = auto()
    Edge131 = auto()
    Edge134 = auto()
    Edge135 = auto()
    Edge136 = auto()
    Edge137 = auto()
    Edge138 = auto()
    Edge139 = auto()
    Edge140 = auto()
    Edge141 = auto()
    Edge142 = auto()
    Edge143 = auto()
    Edge144 = auto()
    Edge145 = auto()
    Edge146 = auto()
    Edge147 = auto()
    Edge148 = auto()

    # Firefox versions
    Firefox109 = auto()
    Firefox117 = auto()
    Firefox128 = auto()
    Firefox133 = auto()
    Firefox135 = auto()
    FirefoxPrivate135 = auto()
    FirefoxAndroid135 = auto()
    Firefox136 = auto()
    FirefoxPrivate136 = auto()
    Firefox139 = auto()
    Firefox142 = auto()
    Firefox143 = auto()
    Firefox144 = auto()
    Firefox145 = auto()
    Firefox146 = auto()
    Firefox147 = auto()
    Firefox148 = auto()
    Firefox149 = auto()
    Firefox150 = auto()
    Firefox151 = auto()

    # Safari versions
    SafariIos17_2 = auto()
    SafariIos17_4_1 = auto()
    SafariIos16_5 = auto()
    Safari15_3 = auto()
    Safari15_5 = auto()
    Safari15_6_1 = auto()
    Safari16 = auto()
    Safari16_5 = auto()
    Safari17_0 = auto()
    Safari17_2_1 = auto()
    Safari17_4_1 = auto()
    Safari17_5 = auto()
    Safari17_6 = auto()
    Safari18 = auto()
    SafariIPad18 = auto()
    Safari18_2 = auto()
    Safari18_3 = auto()
    Safari18_3_1 = auto()
    SafariIos18_1_1 = auto()
    Safari18_5 = auto()
    Safari26 = auto()
    Safari26_1 = auto()
    Safari26_2 = auto()
    Safari26_3 = auto()
    Safari26_4 = auto()
    SafariIos26 = auto()
    SafariIos26_2 = auto()
    SafariIPad26 = auto()
    SafariIpad26_2 = auto()

    # OkHttp versions
    OkHttp3_9 = auto()
    OkHttp3_11 = auto()
    OkHttp3_13 = auto()
    OkHttp3_14 = auto()
    OkHttp4_9 = auto()
    OkHttp4_10 = auto()
    OkHttp4_12 = auto()
    OkHttp5 = auto()

    # Opera versions
    Opera116 = auto()
    Opera117 = auto()
    Opera118 = auto()
    Opera119 = auto()
    Opera120 = auto()
    Opera121 = auto()
    Opera122 = auto()
    Opera123 = auto()
    Opera124 = auto()
    Opera125 = auto()
    Opera126 = auto()
    Opera127 = auto()
    Opera128 = auto()
    Opera129 = auto()
    Opera130 = auto()
    Opera131 = auto()


@final
class Platform(Enum):
    """
    Selects which platform the client should look like.

    This mainly affects platform-specific headers and user-agent details.
    In most cases you can keep the default unless you need to match a
    specific Windows, macOS, Linux, Android, or iOS profile.
    """

    Windows = auto()  # Windows (any version)
    MacOS = auto()  # macOS (any version)
    Linux = auto()  # Linux (any distribution)
    Android = auto()  # Android (mobile)
    IOS = auto()  # iOS (iPhone/iPad)


@final
class Emulation:
    """
    Represents the configuration options for emulating a client profile and platform.

    The `Emulation` struct allows you to configure various aspects of profile and platform
    emulation, including the profile, platform, and whether to enable certain features
    like HTTP/2 or headers.
    """

    # Chrome versions
    Chrome100: ClassVar[Profile] = Profile.Chrome100
    Chrome101: ClassVar[Profile] = Profile.Chrome101
    Chrome104: ClassVar[Profile] = Profile.Chrome104
    Chrome105: ClassVar[Profile] = Profile.Chrome105
    Chrome106: ClassVar[Profile] = Profile.Chrome106
    Chrome107: ClassVar[Profile] = Profile.Chrome107
    Chrome108: ClassVar[Profile] = Profile.Chrome108
    Chrome109: ClassVar[Profile] = Profile.Chrome109
    Chrome110: ClassVar[Profile] = Profile.Chrome110
    Chrome114: ClassVar[Profile] = Profile.Chrome114
    Chrome116: ClassVar[Profile] = Profile.Chrome116
    Chrome117: ClassVar[Profile] = Profile.Chrome117
    Chrome118: ClassVar[Profile] = Profile.Chrome118
    Chrome119: ClassVar[Profile] = Profile.Chrome119
    Chrome120: ClassVar[Profile] = Profile.Chrome120
    Chrome123: ClassVar[Profile] = Profile.Chrome123
    Chrome124: ClassVar[Profile] = Profile.Chrome124
    Chrome126: ClassVar[Profile] = Profile.Chrome126
    Chrome127: ClassVar[Profile] = Profile.Chrome127
    Chrome128: ClassVar[Profile] = Profile.Chrome128
    Chrome129: ClassVar[Profile] = Profile.Chrome129
    Chrome130: ClassVar[Profile] = Profile.Chrome130
    Chrome131: ClassVar[Profile] = Profile.Chrome131
    Chrome132: ClassVar[Profile] = Profile.Chrome132
    Chrome133: ClassVar[Profile] = Profile.Chrome133
    Chrome134: ClassVar[Profile] = Profile.Chrome134
    Chrome135: ClassVar[Profile] = Profile.Chrome135
    Chrome136: ClassVar[Profile] = Profile.Chrome136
    Chrome137: ClassVar[Profile] = Profile.Chrome137
    Chrome138: ClassVar[Profile] = Profile.Chrome138
    Chrome139: ClassVar[Profile] = Profile.Chrome139
    Chrome140: ClassVar[Profile] = Profile.Chrome140
    Chrome141: ClassVar[Profile] = Profile.Chrome141
    Chrome142: ClassVar[Profile] = Profile.Chrome142
    Chrome143: ClassVar[Profile] = Profile.Chrome143
    Chrome144: ClassVar[Profile] = Profile.Chrome144
    Chrome145: ClassVar[Profile] = Profile.Chrome145
    Chrome146: ClassVar[Profile] = Profile.Chrome146
    Chrome147: ClassVar[Profile] = Profile.Chrome147
    Chrome148: ClassVar[Profile] = Profile.Chrome148
    Chrome149: ClassVar[Profile] = Profile.Chrome149

    # Microsoft Edge versions
    Edge101: ClassVar[Profile] = Profile.Edge101
    Edge122: ClassVar[Profile] = Profile.Edge122
    Edge127: ClassVar[Profile] = Profile.Edge127
    Edge131: ClassVar[Profile] = Profile.Edge131
    Edge134: ClassVar[Profile] = Profile.Edge134
    Edge135: ClassVar[Profile] = Profile.Edge135
    Edge136: ClassVar[Profile] = Profile.Edge136
    Edge137: ClassVar[Profile] = Profile.Edge137
    Edge138: ClassVar[Profile] = Profile.Edge138
    Edge139: ClassVar[Profile] = Profile.Edge139
    Edge140: ClassVar[Profile] = Profile.Edge140
    Edge141: ClassVar[Profile] = Profile.Edge141
    Edge142: ClassVar[Profile] = Profile.Edge142
    Edge143: ClassVar[Profile] = Profile.Edge143
    Edge144: ClassVar[Profile] = Profile.Edge144
    Edge145: ClassVar[Profile] = Profile.Edge145
    Edge146: ClassVar[Profile] = Profile.Edge146
    Edge147: ClassVar[Profile] = Profile.Edge147
    Edge148: ClassVar[Profile] = Profile.Edge148

    # Firefox versions
    Firefox109: ClassVar[Profile] = Profile.Firefox109
    Firefox117: ClassVar[Profile] = Profile.Firefox117
    Firefox128: ClassVar[Profile] = Profile.Firefox128
    Firefox133: ClassVar[Profile] = Profile.Firefox133
    Firefox135: ClassVar[Profile] = Profile.Firefox135
    FirefoxPrivate135: ClassVar[Profile] = Profile.FirefoxPrivate135
    FirefoxAndroid135: ClassVar[Profile] = Profile.FirefoxAndroid135
    Firefox136: ClassVar[Profile] = Profile.Firefox136
    FirefoxPrivate136: ClassVar[Profile] = Profile.FirefoxPrivate136
    Firefox139: ClassVar[Profile] = Profile.Firefox139
    Firefox142: ClassVar[Profile] = Profile.Firefox142
    Firefox143: ClassVar[Profile] = Profile.Firefox143
    Firefox144: ClassVar[Profile] = Profile.Firefox144
    Firefox145: ClassVar[Profile] = Profile.Firefox145
    Firefox146: ClassVar[Profile] = Profile.Firefox146
    Firefox147: ClassVar[Profile] = Profile.Firefox147
    Firefox148: ClassVar[Profile] = Profile.Firefox148
    Firefox149: ClassVar[Profile] = Profile.Firefox149
    Firefox150: ClassVar[Profile] = Profile.Firefox150
    Firefox151: ClassVar[Profile] = Profile.Firefox151

    # Safari versions
    SafariIos17_2: ClassVar[Profile] = Profile.SafariIos17_2
    SafariIos17_4_1: ClassVar[Profile] = Profile.SafariIos17_4_1
    SafariIos16_5: ClassVar[Profile] = Profile.SafariIos16_5
    Safari15_3: ClassVar[Profile] = Profile.Safari15_3
    Safari15_5: ClassVar[Profile] = Profile.Safari15_5
    Safari15_6_1: ClassVar[Profile] = Profile.Safari15_6_1
    Safari16: ClassVar[Profile] = Profile.Safari16
    Safari16_5: ClassVar[Profile] = Profile.Safari16_5
    Safari17_0: ClassVar[Profile] = Profile.Safari17_0
    Safari17_2_1: ClassVar[Profile] = Profile.Safari17_2_1
    Safari17_4_1: ClassVar[Profile] = Profile.Safari17_4_1
    Safari17_5: ClassVar[Profile] = Profile.Safari17_5
    Safari17_6: ClassVar[Profile] = Profile.Safari17_6
    Safari18: ClassVar[Profile] = Profile.Safari18
    SafariIPad18: ClassVar[Profile] = Profile.SafariIPad18
    Safari18_2: ClassVar[Profile] = Profile.Safari18_2
    Safari18_3: ClassVar[Profile] = Profile.Safari18_3
    Safari18_3_1: ClassVar[Profile] = Profile.Safari18_3_1
    SafariIos18_1_1: ClassVar[Profile] = Profile.SafariIos18_1_1
    Safari18_5: ClassVar[Profile] = Profile.Safari18_5
    Safari26: ClassVar[Profile] = Profile.Safari26
    Safari26_1: ClassVar[Profile] = Profile.Safari26_1
    Safari26_2: ClassVar[Profile] = Profile.Safari26_2
    Safari26_3: ClassVar[Profile] = Profile.Safari26_3
    Safari26_4: ClassVar[Profile] = Profile.Safari26_4
    SafariIos26: ClassVar[Profile] = Profile.SafariIos26
    SafariIos26_2: ClassVar[Profile] = Profile.SafariIos26_2
    SafariIPad26: ClassVar[Profile] = Profile.SafariIPad26
    SafariIpad26_2: ClassVar[Profile] = Profile.SafariIpad26_2

    # OkHttp versions
    OkHttp3_9: ClassVar[Profile] = Profile.OkHttp3_9
    OkHttp3_11: ClassVar[Profile] = Profile.OkHttp3_11
    OkHttp3_13: ClassVar[Profile] = Profile.OkHttp3_13
    OkHttp3_14: ClassVar[Profile] = Profile.OkHttp3_14
    OkHttp4_9: ClassVar[Profile] = Profile.OkHttp4_9
    OkHttp4_10: ClassVar[Profile] = Profile.OkHttp4_10
    OkHttp4_12: ClassVar[Profile] = Profile.OkHttp4_12
    OkHttp5: ClassVar[Profile] = Profile.OkHttp5

    # Opera versions
    Opera116: ClassVar[Profile] = Profile.Opera116
    Opera117: ClassVar[Profile] = Profile.Opera117
    Opera118: ClassVar[Profile] = Profile.Opera118
    Opera119: ClassVar[Profile] = Profile.Opera119
    Opera120: ClassVar[Profile] = Profile.Opera120
    Opera121: ClassVar[Profile] = Profile.Opera121
    Opera122: ClassVar[Profile] = Profile.Opera122
    Opera123: ClassVar[Profile] = Profile.Opera123
    Opera124: ClassVar[Profile] = Profile.Opera124
    Opera125: ClassVar[Profile] = Profile.Opera125
    Opera126: ClassVar[Profile] = Profile.Opera126
    Opera127: ClassVar[Profile] = Profile.Opera127
    Opera128: ClassVar[Profile] = Profile.Opera128
    Opera129: ClassVar[Profile] = Profile.Opera129
    Opera130: ClassVar[Profile] = Profile.Opera130
    Opera131: ClassVar[Profile] = Profile.Opera131

    def __init__(
        self,
        profile: Profile = Profile.Chrome100,
        platform: Platform = Platform.MacOS,
        http2: bool = True,
        headers: bool = True,
    ) -> None:
        """
        Create a new emulation configuration.

        Args:
            profile: Whether to change the profile (browser/okhttp) information.
            platform: Whether to change the platform (Windows/macOS/Linux/Android/iOS) information.
            http2: Whether to enable HTTP/2.
            headers: Whether to include default headers.

        Returns:
            A configured Emulation instance

        Example:
            ```python
            # Chrome on Windows with HTTP/2 disabled
            option = Emulation(
                profile=Profile.Chrome137,
                platform=Platform.Windows,
                http2=False,
                headers=True
            )
            ```
        """
        ...

    @staticmethod
    def random() -> "Emulation":
        """
        Generate a random emulation configuration.

        Example:
            ```python
            # Use different random emulation for each client
            client = wreq.Client(emulation=Emulation.random())
            ```
        """
        ...
