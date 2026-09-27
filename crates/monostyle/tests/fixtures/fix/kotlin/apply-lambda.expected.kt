data class Config(var retries: Int = 0, var label: String = "")

fun tuned(): Config {
    val config = Config().apply {
        retries = 3
        label = "tuned {kept}"
    }

    return config
}
