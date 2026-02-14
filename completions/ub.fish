function __ub_projects
	set -l config_path "$HOME/.config/ubermind/projects"
	if set -q XDG_CONFIG_HOME
		set config_path "$XDG_CONFIG_HOME/ubermind/projects"
	end

	if test -f $config_path
		grep -v '^#' $config_path 2>/dev/null | grep -v '^[[:space:]]*$' | cut -d: -f1 | sed 's/^[[:space:]]*//;s/[[:space:]]*$//'
	end
end

complete -c ub -f
complete -c ubermind -f

complete -c ub -n "__fish_use_subcommand" -a "status" -d "show project status"
complete -c ub -n "__fish_use_subcommand" -a "st" -d "show project status (alias)"
complete -c ub -n "__fish_use_subcommand" -a "start" -d "start project(s)"
complete -c ub -n "__fish_use_subcommand" -a "stop" -d "stop project(s)"
complete -c ub -n "__fish_use_subcommand" -a "reload" -d "restart project(s)"
complete -c ub -n "__fish_use_subcommand" -a "kill" -d "kill process(es)"
complete -c ub -n "__fish_use_subcommand" -a "echo" -d "view logs"
complete -c ub -n "__fish_use_subcommand" -a "connect" -d "connect to process"
complete -c ub -n "__fish_use_subcommand" -a "restart" -d "restart process(es)"
complete -c ub -n "__fish_use_subcommand" -a "quit" -d "quit overmind"
complete -c ub -n "__fish_use_subcommand" -a "run" -d "run command"
complete -c ub -n "__fish_use_subcommand" -a "init" -d "create config file"
complete -c ub -n "__fish_use_subcommand" -a "add" -d "add a project"
complete -c ub -n "__fish_use_subcommand" -a "serve" -d "start web UI"
complete -c ub -n "__fish_use_subcommand" -a "ui" -d "start web UI (alias)"
complete -c ub -n "__fish_use_subcommand" -a "help" -d "show help"
complete -c ub -n "__fish_use_subcommand" -a "version" -d "show version"

complete -c ub -n "__fish_use_subcommand" -a "(__ub_projects)"

complete -c ub -n "__fish_seen_subcommand_from status st start stop reload kill echo connect restart quit run" -a "(__ub_projects)"
complete -c ub -n "__fish_seen_subcommand_from status st start stop reload kill echo connect restart quit run" -l all -d "target all projects"
complete -c ub -n "__fish_seen_subcommand_from status st start stop reload kill echo connect restart quit run" -s a -d "target all projects"

complete -c ub -n "__fish_seen_subcommand_from serve ui" -l daemon -d "run in background"
complete -c ub -n "__fish_seen_subcommand_from serve ui" -s d -d "run in background"
complete -c ub -n "__fish_seen_subcommand_from serve ui" -l stop -d "stop daemon"
complete -c ub -n "__fish_seen_subcommand_from serve ui" -l echo -d "view daemon logs"
complete -c ub -n "__fish_seen_subcommand_from serve ui" -l restart -d "restart daemon"
complete -c ub -n "__fish_seen_subcommand_from serve ui" -l status -d "show daemon status"

complete -c ub -s h -l help -d "show help"
complete -c ub -s V -l version -d "show version"

complete -c ubermind -n "__fish_use_subcommand" -a "status st start stop reload kill echo connect restart quit run init add serve ui help version"
complete -c ubermind -n "__fish_use_subcommand" -a "(__ub_projects)"
complete -c ubermind -n "__fish_seen_subcommand_from status st start stop reload kill echo connect restart quit run" -a "(__ub_projects)"
complete -c ubermind -n "__fish_seen_subcommand_from status st start stop reload kill echo connect restart quit run" -l all -d "target all projects"
