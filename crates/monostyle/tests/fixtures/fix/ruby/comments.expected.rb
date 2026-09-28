# frozen_string_literal: true

# Registry of scanned targets.
class Registry
  # Skip when the target is missing.
  def scan(target)
    return nil if target.nil?

    target.read
  end
end
