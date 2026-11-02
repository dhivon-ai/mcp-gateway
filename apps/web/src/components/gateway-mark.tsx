import mark from "../../../../assets/brand/logo.svg?no-inline";
import darkMark from "../../../../assets/brand/logo-dark.svg?no-inline";

export function GatewayMark() {
  return (
    <span className="gateway-mark" aria-hidden="true">
      <img
        className="gateway-mark-light"
        src={mark}
        alt=""
        width="44"
        height="44"
      />
      <img
        className="gateway-mark-dark"
        src={darkMark}
        alt=""
        width="44"
        height="44"
      />
    </span>
  );
}
