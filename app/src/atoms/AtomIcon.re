module Logo = {
  [@react.component]
  let make = () =>
    <svg
      width="24"
      height="24"
      viewBox="0 0 24 24"
      fill="none"
      xmlns="http://www.w3.org/2000/svg">
      <circle cx="12" cy="12" r="11" stroke="#28333D" strokeWidth="2" />
    </svg>;
};

module Close = {
  [@react.component]
  let make = () =>
    <svg
      width="34"
      height="34"
      viewBox="0 0 34 34"
      fill="none"
      xmlns="http://www.w3.org/2000/svg">
      <path
        fillRule="evenodd"
        clipRule="evenodd"
        d="M25.4853 8.5147C25.0948 8.12418 24.4616 8.12418 24.0711 8.5147L17 15.5858L9.92893 8.5147C9.53841 8.12418 8.90524 8.12418 8.51472 8.5147C8.12419 8.90522 8.12419 9.53839 8.51472 9.92891L15.5858 17L8.51472 24.071C8.12419 24.4616 8.12419 25.0947 8.51472 25.4853C8.90524 25.8758 9.53841 25.8758 9.92893 25.4853L17 18.4142L24.0711 25.4853C24.4616 25.8758 25.0948 25.8758 25.4853 25.4853C25.8758 25.0947 25.8758 24.4616 25.4853 24.071L18.4142 17L25.4853 9.92891C25.8758 9.53839 25.8758 8.90522 25.4853 8.5147Z"
        fill="#90A0AF"
      />
    </svg>;
};

module Back = {
  [@react.component]
  let make = () =>
    <svg
      width="16"
      height="16"
      viewBox="0 0 16 16"
      fill="none"
      xmlns="http://www.w3.org/2000/svg">
      <path
        d="M10 4L6 8L10 12"
        stroke="#90A0AF"
        strokeWidth="2"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>;
};

module PersonAvatarPlaceholder = {
  [@react.component]
  let make = () =>
    <svg
      width="36"
      height="36"
      viewBox="0 0 36 36"
      fill="none"
      xmlns="http://www.w3.org/2000/svg">
      <circle cx="18" cy="18" r="18" fill="url(#paint0_radial)" />
      <defs>
        <radialGradient
          id="paint0_radial"
          cx="0"
          cy="0"
          r="1"
          gradientUnits="userSpaceOnUse"
          gradientTransform="translate(18 18) rotate(90) scale(55.5)">
          <stop stopColor="#E074CB" />
          <stop offset="1" stopColor="#6E41E0" />
        </radialGradient>
      </defs>
    </svg>;
};

module ProjectAvatarPlaceholder = {
  [@react.component]
  let make = () =>
    <svg
      width="64"
      height="64"
      viewBox="0 0 64 64"
      fill="none"
      xmlns="http://www.w3.org/2000/svg">
      <rect width="64" height="64" rx="2" fill="url(#paint0_radial)" />
      <defs>
        <radialGradient
          id="paint0_radial"
          cx="0"
          cy="0"
          r="1"
          gradientUnits="userSpaceOnUse"
          gradientTransform="translate(32 32) rotate(90) scale(98.6667)">
          <stop stopColor="#E074CB" />
          <stop offset="1" stopColor="#6E41E0" />
        </radialGradient>
      </defs>
    </svg>;
};
