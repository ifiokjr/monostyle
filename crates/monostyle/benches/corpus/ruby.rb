# frozen_string_literal: true

# Order fulfilment with the Ruby shapes that trip a scanner: `#{}` interpolation holding quotes,
# squiggly heredocs, modifier conditionals, and `=begin`/`=end` comments.

require "json"

module Fulfilment
  SHIPPING = { standard: 5.0, express: 15.0, overnight: 30.0 }.freeze

  # A line on an order.
  Line = Struct.new(:sku, :quantity, :price) do
    def total
      quantity * price
    end
  end

  class Order
    attr_reader :id, :lines, :shipping

    def initialize(id, lines, shipping: :standard)
      @id = id
      @lines = lines
      @shipping = shipping
    end

    def subtotal
      lines.sum(&:total)
    end

    def total
      subtotal + SHIPPING.fetch(shipping) { raise ArgumentError, "unknown shipping #{shipping.inspect}" }
    end

    def summary
      "Order ##{id}: #{lines.size} line#{'s' unless lines.size == 1}, total #{format('%.2f', total)}"
    end

    def to_sql
      <<~SQL
        INSERT INTO orders (id, subtotal, shipping)
        VALUES (#{id}, #{subtotal}, '#{shipping}');
      SQL
    end

    def receipt
      <<~'TEXT'
        Thank you for your order.
        Your total is shown above; #{this} is not interpolated.
      TEXT
    end
  end

=begin
  A block comment: the scanner must treat everything here as commentary,
  including things that look like code: if x then y end
=end

  class Picker
    def initialize(stock)
      @stock = stock
    end

    def pick(order)
      order.lines.each_with_object([]) do |line, picked|
        next if line.quantity.zero?

        available = @stock.fetch(line.sku, 0)

        if available >= line.quantity
          @stock[line.sku] = available - line.quantity
          picked << line
        elsif available.positive?
          picked << Line.new(line.sku, available, line.price)
          @stock[line.sku] = 0
        else
          warn "out of stock: #{line.sku}" unless ENV["QUIET"]
        end
      end
    end

    def restock(sku, quantity)
      raise ArgumentError, "quantity must be positive, got #{quantity}" unless quantity.positive?

      @stock[sku] = (@stock[sku] || 0) + quantity
    rescue ArgumentError => error
      warn error.message
      nil
    end
  end

  def self.describe(orders)
    orders.map do |order|
      case order.shipping
      when :overnight then "#{order.summary} (rush)"
      when :express then "#{order.summary} (fast)"
      else order.summary
      end
    end
  end
end
