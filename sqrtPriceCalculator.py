from decimal import Decimal, getcontext
getcontext().prec = 80           # plenty for 256-bit math

Q96 = 2 ** 96

def sqrt_price_x96_from_price(token_per_weth: float,
                              decimals_token: int = 18,
                              decimals_weth:  int = 18):
    """
    token_per_weth – human-readable price (how many TOKEN for 1 WETH)
    Returns (_POOL_SQRT_PRICE_X96_WETH_0, _POOL_SQRT_PRICE_X96_TOKEN_0)
    """

    P = Decimal(str(token_per_weth))
    if P <= 0:
        raise ValueError("Price must be positive")

    # ── WETH is token0 ──────────────────────────────────────────────
    price_weth0  = P * (Decimal(10) ** decimals_token) / (Decimal(10) ** decimals_weth)
    sqrt_weth0   = (price_weth0.sqrt() * Decimal(Q96)).to_integral_value(rounding="ROUND_FLOOR")
    sqrt_x96_w0  = int(sqrt_weth0)

    # ── TOKEN is token0 (inverse price) ─────────────────────────────
    price_token0 = (Decimal(1) / P) * (Decimal(10) ** decimals_weth) / (Decimal(10) ** decimals_token)
    sqrt_token0  = (price_token0.sqrt() * Decimal(Q96)).to_integral_value(rounding="ROUND_FLOOR")
    sqrt_x96_t0  = int(sqrt_token0)

    return sqrt_x96_w0, sqrt_x96_t0

if __name__ == "__main__":
    # both 18-dec tokens ⇒ result is 2**96 when price==1
    print(sqrt_price_x96_from_price(8000000))

    # 3 000 USDC/WETH (USDC 6 decimals)
    print(sqrt_price_x96_from_price(3_000, decimals_token=6))
