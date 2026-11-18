"""Invoice numbering and formatting.

The f-strings here nest braces, repeat width specifiers, and escape literal braces, which is the
part of Python a line-oriented scanner most often gets wrong.
"""

from __future__ import annotations

import re
from dataclasses import dataclass, field
from datetime import date
from decimal import Decimal

#: Invoice numbers are a prefix, a year, and a zero-padded sequence.
PATTERN = re.compile(r"^(?P<prefix>[A-Z]{2,4})-(?P<year>\d{4})-(?P<sequence>\d{5})$")

RAW_TEMPLATE = r"C:\invoices\{year}\{sequence}.pdf"
ENCODED = b"\x89INV\r\n"


@dataclass(frozen=True)
class Line:
    """One billed line."""

    description: str
    quantity: int
    unit_price: Decimal

    @property
    def total(self) -> Decimal:
        return self.unit_price * self.quantity


@dataclass
class Invoice:
    """An invoice with its lines and a running number."""

    prefix: str
    issued: date
    sequence: int
    lines: list[Line] = field(default_factory=list)

    @property
    def number(self) -> str:
        return f"{self.prefix}-{self.issued:%Y}-{self.sequence:05d}"

    @property
    def total(self) -> Decimal:
        return sum((line.total for line in self.lines), Decimal(0))

    def render(self, width: int = 48) -> str:
        """Renders the invoice as fixed-width text.

        The width of each column is itself interpolated, so the format spec contains a nested
        replacement field, and the literal braces around the total are doubled.
        """
        header = f"{self.number:<{width}}{self.issued:%d %b %Y}"
        rows = [
            f"{line.description[:width - 12]:<{width - 12}} {line.quantity:>3} x {line.unit_price:>7.2f}"
            for line in self.lines
        ]
        footer = f"{{total}} = {self.total:.2f}"

        return "\n".join([header, *rows, footer])


def parse(number: str) -> tuple[str, int, int]:
    """Splits an invoice number into its parts, or raises."""
    match = PATTERN.match(number)

    if match is None:
        raise ValueError(f"{number!r} is not an invoice number")

    return match["prefix"], int(match["year"]), int(match["sequence"])


def next_number(existing: list[str], prefix: str, today: date) -> str:
    # The sequence restarts every year, so only this year's numbers count.
    this_year = [
        parse(number)[2]
        for number in existing
        if number.startswith(f"{prefix}-{today:%Y}-")
    ]
    sequence = max(this_year, default=0) + 1

    try:
        return f"{prefix}-{today:%Y}-{sequence:05d}"
    except ValueError as error:
        raise RuntimeError("sequence overflow") from error


def describe(invoices: list[Invoice]) -> dict[str, str]:
    return {
        invoice.number: (
            f"{len(invoice.lines)} line{'s' if len(invoice.lines) != 1 else ''}, "
            f"total {invoice.total:,.2f}"
        )
        for invoice in invoices
        if invoice.lines
    }
