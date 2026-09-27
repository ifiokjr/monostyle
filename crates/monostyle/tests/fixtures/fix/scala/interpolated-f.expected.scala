object Report {
  def label(count: Int, ratio: Double): String = {
    val text = f"$count%d items at $ratio%.1f%% {kept}"

    text
  }
}
