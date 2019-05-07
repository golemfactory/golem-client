#! /bin/bash
#
# golemcli installer updater
#

get_latest_release() {
	curl --silent "https://api.github.com/repos/$1/releases/latest" | # Get latest release from GitHub api
  	grep '"tag_name":' |                                            # Get tag line
  	sed -E 's/.*"([^"]+)".*/\1/'                                    # Pluck JSON value
}

fail() {
	echo $@ >&2
	exit 1
}

os_name() {
	case "$(uname | tr '[:upper:]' '[:lower:]')" in
  		linux*)
	    		echo -n linux
    			;;
		darwin*)
			echo -n osx
    			;;
  		msys*)
    			echo -n windows
    			;;
  		*)
    			fail "system not suported"
    			;;
	esac
}

message() {
	echo $@
}

check_tool() {
	local TOOLBIN=$1
	local TOOLPATH=$(which "$TOOLBIN")
	if [ -z "$TOOLPATH" ]; then
		fail "$TOOLBIN not found"
	fi
}


install_golemcli() {
	message "-== GOLEM CLI DEV Update ==-"

	check_tool curl
	check_tool awk
	
	local OS_NAME=$(os_name)
	local TAG=$(get_latest_release golemfactory/golem-client)

	[ "$OS_NAME" != "windows" ] || check_tool 7z

	local DIST_NAME=golemcli-$(os_name)-${TAG}
	local CURENT_PATH=$(which golemcli)
	local CP=cp

	if [ -n "${CURENT_PATH}" ]; then
		message "golemcli already found"
		message "current path:          ${CURENT_PATH}"
		CLIVER=$(golemcli 2>/dev/null| awk 'NR == 1 && $1 == "golemcli" { print $2 }')
		message "current version:       ${CLIVER:-unknown oldcli}"
		message "new version:           ${TAG}"
		
		read -p "override (y/N): " Q
		while [ "$Q" != "y" ] && [ "${Q:-n}" != "n" ]; do
			echo wrong answer \"$Q\"
			read -p "override (y/N): " Q
		done
		[ "${Q:-n}" = "n" ] && exit 0
		test -w "$CURENT_PATH" || CP="sudo cp"
	else
		CURENT_PATH="$HOME/bin/golemcli"
		message "installing to $CURENT_PATH"
	fi

	local UPDATE_WORK_DIR="$(mktemp -d)"
	trap "rm -rf $UPDATE_WORK_DIR" EXIT

	echo -n "download ${DIST_NAME}.tar.gz  "
	curl -sSL https://github.com/golemfactory/golem-client/releases/download/${TAG}/${DIST_NAME}.tar.gz | tar xz -C "${UPDATE_WORK_DIR}" -f - 
	echo " [ done ] "

	"$UPDATE_WORK_DIR/$DIST_NAME/golemcli" _int complete bash > $UPDATE_WORK_DIR/golemcli-complete.sh
	echo -n "installing to $CURENT_PATH   "
	$CP "$UPDATE_WORK_DIR/$DIST_NAME/golemcli" "$CURENT_PATH"
	echo " [ done ] "

	if test -d /etc/bash_completion.d/; then
		read -p "install autocomplete definitions for bash (y/N): " Q
		while [ "$Q" != "y" ] && [ "${Q:-n}" != "n" ]; do
			echo wrong answer \"$Q\"
			read -p "install autocomplete definitions for bash (y/N): " Q
		done
		[ "${Q:-n}" = "n" ] && exit 0
		sudo cp "$UPDATE_WORK_DIR/golemcli-complete.sh" /etc/bash_completion.d/golemcli
	fi
}
		
install_golemcli </dev/tty

