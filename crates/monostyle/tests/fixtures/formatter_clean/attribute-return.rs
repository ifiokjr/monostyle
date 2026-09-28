fn command(&self) -> Command {
	#[cfg(test)]
	return Command::new(&self.path);

	#[cfg(not(test))]
	return self.build();
}
