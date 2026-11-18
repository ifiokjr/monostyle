<?php

declare(strict_types=1);

namespace App\Billing;

/**
 * Builds invoices from a cart. The heredoc and nowdoc below, and the two interpolation syntaxes
 * inside double-quoted strings, are the PHP shapes a scanner has to tell apart.
 */
final class InvoiceBuilder
{
    private const TAX_RATE = 0.2;

    /** @var array<int, array{sku: string, quantity: int, price: float}> */
    private array $lines = [];

    public function __construct(private readonly string $currency = 'EUR')
    {
    }

    public function add(string $sku, int $quantity, float $price): self
    {
        if ($quantity <= 0) {
            throw new \InvalidArgumentException("quantity for {$sku} must be positive, got $quantity");
        }

        $this->lines[] = ['sku' => $sku, 'quantity' => $quantity, 'price' => $price];

        return $this;
    }

    public function subtotal(): float
    {
        return array_sum(array_map(static fn(array $line): float => $line['quantity'] * $line['price'], $this->lines));
    }

    public function total(): float
    {
        return $this->subtotal() * (1 + self::TAX_RATE);
    }

    # A heredoc: variables and {$expressions} are interpolated until the terminator line.
    public function render(?string $customer): string
    {
        $name = $customer ?? 'customer';
        $count = count($this->lines);
        $total = number_format($this->total(), 2);

        return <<<EOT
        Dear {$name},

        Your invoice has {$count} line(s) and comes to {$total} {$this->currency}.
        Tax is included at {${'rate'}} percent.
        EOT;
    }

    // A nowdoc: nothing is interpolated, so the dollar signs are content.
    public function terms(): string
    {
        return <<<'TERMS'
        Payment is due within 30 days. Late payments accrue $5 per day, up to {$cap}.
        TERMS;
    }

    public function describe(): string
    {
        return match (true) {
            $this->lines === [] => 'empty invoice',
            count($this->lines) === 1 => 'single-line invoice',
            default => sprintf('%d-line invoice', count($this->lines)),
        };
    }

    public function heaviestLine(): ?array
    {
        $heaviest = null;

        foreach ($this->lines as $line) {
            if ($heaviest === null || $line['quantity'] * $line['price'] > $heaviest['quantity'] * $heaviest['price']) {
                $heaviest = $line;
            }
        }

        return $heaviest;
    }

    public function skuOfHeaviest(): string
    {
        return $this->heaviestLine()?->offsetGet('sku') ?? 'none';
    }
}
