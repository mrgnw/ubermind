#!/bin/sh
set -eu

repo="mrgnw/ubermind"
bin="ubermind"
overmind_repo="DarthSim/overmind"
install_dir="${INSTALL_DIR:-${HOME}/.local/bin}"

detect_target() {
	os=$(uname -s)
	arch=$(uname -m)

	case "${os}" in
		Darwin) os_part="apple-darwin" ;;
		Linux)  os_part="unknown-linux-musl" ;;
		*)
			echo "unsupported OS: ${os}" >&2
			exit 1
			;;
	esac

	case "${arch}" in
		x86_64|amd64)  arch_part="x86_64" ;;
		arm64|aarch64) arch_part="aarch64" ;;
		*)
			echo "unsupported architecture: ${arch}" >&2
			exit 1
			;;
	esac

	echo "${arch_part}-${os_part}"
}

latest_tag() {
	if command -v curl >/dev/null 2>&1; then
		curl -fsSL "https://api.github.com/repos/${repo}/releases/latest" | grep '"tag_name"' | sed 's/.*"tag_name": *"\([^"]*\)".*/\1/'
	elif command -v wget >/dev/null 2>&1; then
		wget -qO- "https://api.github.com/repos/${repo}/releases/latest" | grep '"tag_name"' | sed 's/.*"tag_name": *"\([^"]*\)".*/\1/'
	else
		echo "curl or wget required" >&2
		exit 1
	fi
}

download() {
	url="$1"
	dest="$2"
	if command -v curl >/dev/null 2>&1; then
		curl -fsSL -o "${dest}" "${url}"
	else
		wget -qO "${dest}" "${url}"
	fi
}

target=$(detect_target)
tag=$(latest_tag)
archive="${bin}-${tag}-${target}.tar.gz"
url="https://github.com/${repo}/releases/download/${tag}/${archive}"

echo "installing ${bin} ${tag} (${target})"

tmpdir=$(mktemp -d)
trap 'rm -rf "${tmpdir}"' EXIT

download "${url}" "${tmpdir}/${archive}"
tar -xzf "${tmpdir}/${archive}" -C "${tmpdir}"

mkdir -p "${install_dir}"
mv "${tmpdir}/${bin}" "${install_dir}/${bin}"
chmod +x "${install_dir}/${bin}"
ln -sf "${install_dir}/${bin}" "${install_dir}/ub"

echo "installed ${install_dir}/${bin}"
echo "created alias ${install_dir}/ub"

# --- install tmux if missing ---

if command -v tmux >/dev/null 2>&1; then
	echo "tmux already installed: $(tmux -V 2>&1)"
else
	echo
	echo "tmux not found (required by overmind)"

	os=$(uname -s)
	case "${os}" in
		Darwin)
			if command -v brew >/dev/null 2>&1; then
				echo "installing tmux via brew..."
				brew install tmux
			else
				echo "please install tmux: brew install tmux"
				exit 1
			fi
			;;
		Linux)
			if command -v apt-get >/dev/null 2>&1; then
				echo "installing tmux via apt-get..."
				sudo apt-get install -y tmux
			elif command -v dnf >/dev/null 2>&1; then
				echo "installing tmux via dnf..."
				sudo dnf install -y tmux
			elif command -v yum >/dev/null 2>&1; then
				echo "installing tmux via yum..."
				sudo yum install -y tmux
			elif command -v apk >/dev/null 2>&1; then
				echo "installing tmux via apk..."
				apk add tmux
			else
				echo "please install tmux manually"
				exit 1
			fi
			;;
		*)
			echo "please install tmux manually"
			exit 1
			;;
	esac

	if command -v tmux >/dev/null 2>&1; then
		echo "tmux installed: $(tmux -V 2>&1)"
	else
		echo "tmux installation failed"
		exit 1
	fi
fi

# --- install overmind if missing ---

if command -v overmind >/dev/null 2>&1; then
	echo "overmind already installed: $(overmind --version 2>&1 | head -1)"
else
	echo
	echo "installing overmind..."

	os=$(uname -s)
	arch=$(uname -m)

	case "${os}" in
		Darwin) om_os="macos" ;;
		Linux)  om_os="linux" ;;
		*)
			echo "unsupported OS for overmind: ${os}" >&2
			echo "install manually: https://github.com/${overmind_repo}"
			exit 0
			;;
	esac

	case "${arch}" in
		x86_64|amd64)  om_arch="amd64" ;;
		arm64|aarch64) om_arch="arm64" ;;
		*)
			echo "unsupported arch for overmind: ${arch}" >&2
			echo "install manually: https://github.com/${overmind_repo}"
			exit 0
			;;
	esac

	if command -v curl >/dev/null 2>&1; then
		om_tag=$(curl -fsSL "https://api.github.com/repos/${overmind_repo}/releases/latest" | grep '"tag_name"' | sed 's/.*"\(.*\)".*/\1/')
	elif command -v wget >/dev/null 2>&1; then
		om_tag=$(wget -qO- "https://api.github.com/repos/${overmind_repo}/releases/latest" | grep '"tag_name"' | sed 's/.*"\(.*\)".*/\1/')
	fi
	om_tag="${om_tag:-v2.5.1}"

	om_url="https://github.com/${overmind_repo}/releases/download/${om_tag}/overmind-${om_tag}-${om_os}-${om_arch}.gz"
	echo "downloading ${om_url}"

	om_tmp=$(mktemp)
	trap 'rm -f "${om_tmp}"' EXIT

	download "${om_url}" "${om_tmp}"
	gunzip -f "${om_tmp}"
	# gunzip strips .gz, but mktemp has no extension so the output is at the same path
	# handle both cases
	if [ -f "${om_tmp}" ]; then
		mv "${om_tmp}" "${install_dir}/overmind"
	else
		mv "${om_tmp%.gz}" "${install_dir}/overmind" 2>/dev/null || mv "${om_tmp}" "${install_dir}/overmind"
	fi
	chmod +x "${install_dir}/overmind"

	echo "installed overmind ${om_tag} to ${install_dir}/overmind"
fi

# --- install shell completions ---

completion_dir="${HOME}/.local/share/ubermind/completions"
mkdir -p "${completion_dir}"

for shell in bash zsh fish; do
	completion_url="https://raw.githubusercontent.com/${repo}/${tag}/completions/ub.${shell}"
	download "${completion_url}" "${completion_dir}/ub.${shell}" 2>/dev/null || true
done

if [ -f "${completion_dir}/ub.bash" ]; then
	echo
	echo "shell completions installed to ${completion_dir}"
	echo
	echo "enable tab completion:"
	echo
	echo "  bash:"
	echo "    echo 'source ${completion_dir}/ub.bash' >> ~/.bashrc"
	echo
	echo "  zsh:"
	echo "    echo 'fpath=(${completion_dir} \$fpath)' >> ~/.zshrc"
	echo "    echo 'autoload -Uz compinit && compinit' >> ~/.zshrc"
	echo
	echo "  fish:"
	echo "    ln -s ${completion_dir}/ub.fish ~/.config/fish/completions/"
fi

# --- PATH hint ---

if ! echo "${PATH}" | tr ':' '\n' | grep -qx "${install_dir}"; then
	echo
	echo "add to your PATH:"
	echo "  export PATH=\"${install_dir}:\${PATH}\""
fi
